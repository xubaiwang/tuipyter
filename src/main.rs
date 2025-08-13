//! This is the example from `runtimelib`, but changed to use deno.
use std::{env, io::Write};

use r3bl_tui::{ReadlineAsyncContext, ReadlineEvent};
use runtimelib::{
    ConnectionInfo, ExecuteRequest, ExecutionState, JupyterMessage, JupyterMessageContent,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 内核发现与选择：从系统文件夹中发现内核列表并选择 deno
    let kernel_name = "deno";
    let kernelspecs = runtimelib::list_kernelspecs().await;
    // the deno kernel
    let kernel_specification = kernelspecs.get(0).unwrap();

    // 创建连接：需要五个通信接口
    let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
    let ports = runtimelib::peek_ports(ip, 5).await?;
    let connection_info = ConnectionInfo {
        transport: jupyter_protocol::connection_info::Transport::TCP,
        ip: ip.to_string(),
        stdin_port: ports[0],
        control_port: ports[1],
        hb_port: ports[2],
        shell_port: ports[3],
        iopub_port: ports[4],
        signature_scheme: "hmac-sha256".to_string(),
        key: uuid::Uuid::new_v4().to_string(),
        kernel_name: Some(kernel_name.to_string()),
    };

    // 序列化连接信息并交付在运行时文件夹
    let runtime_dir = runtimelib::dirs::runtime_dir();
    tokio::fs::create_dir_all(&runtime_dir).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to create jupyter runtime dir {}: {}",
            runtime_dir.display(),
            e
        )
    })?;
    let connection_path = runtime_dir.join("kernel-example.json");
    let content = serde_json::to_string(&connection_info)?;
    tokio::fs::write(connection_path.clone(), content).await?;

    // 为什么需要？保证比如相对文件路径不出错
    // 直接设置为当前路径即可
    // 启动内核作为新进程
    let working_directory = env::current_dir().unwrap();
    let mut process = kernel_specification
        .clone()
        .command(&connection_path, None, None)?
        .current_dir(working_directory)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let session_id = Uuid::new_v4().to_string();

    // 创建连接，监听数据
    // Listen for display data, execute result, stdout messages, etc.
    let mut iopub_socket =
        runtimelib::create_client_iopub_connection(&connection_info, "", &session_id).await?;
    let mut shell_socket =
        runtimelib::create_client_shell_connection(&connection_info, &session_id).await?;
    // Control socket is for kernel management, not used here
    // let mut control_socket =
    //     runtimelib::create_client_control_connection(&connection_info, &session_id).await?;

    // 在初始化连接之后再创建 REPL
    let mut rl_ctx = ReadlineAsyncContext::try_new(Some("> "))
        .await
        .unwrap()
        .unwrap();
    let mut shared_writer = rl_ctx.clone_shared_writer();
    let ReadlineAsyncContext {
        readline: ref mut rl,
        ..
    } = rl_ctx;

    loop {
        match rl.readline().await {
            Ok(event) => {
                match event {
                    ReadlineEvent::Line(line) => {
                        // 执行请求
                        let execute_request = ExecuteRequest::new(line);
                        let execute_request: JupyterMessage = execute_request.into();
                        let execute_request_id = execute_request.header.msg_id.clone();

                        shell_socket.send(execute_request).await?;

                        loop {
                            // 监听状态
                            match iopub_socket.read().await {
                                Ok(message) => match message.content {
                                    JupyterMessageContent::Status(status) => {
                                        // 等待空闲退出
                                        if status.execution_state == ExecutionState::Idle
                                            && message
                                                .parent_header
                                                .as_ref()
                                                .map(|h| h.msg_id.as_str())
                                                == Some(execute_request_id.as_str())
                                        {
                                            writeln!(
                                                shared_writer,
                                                "Execution finalized, exiting..."
                                            )
                                            .unwrap();
                                            break;
                                        }
                                    }
                                    _ => {
                                        writeln!(shared_writer, "{:?}", message.content).unwrap();
                                    }
                                },
                                Err(e) => {
                                    writeln!(shared_writer, "Error receiving iopub message: {}", e)
                                        .unwrap();
                                    break;
                                }
                            }
                        }
                    }
                    _ => break,
                }
            }
            Err(_) => break,
        }
    }

    process.start_kill()?;

    rl_ctx
        .request_shutdown(Some("Shutting down..."))
        .await
        .unwrap();
    rl_ctx.await_shutdown().await;
    Ok(())
}
