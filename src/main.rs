//! This is the example from `runtimelib`, but changed to use deno.
use runtimelib::{
    ConnectionInfo, ExecuteRequest, ExecutionState, JupyterMessage, JupyterMessageContent,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let kernel_name = "deno";
    let kernelspecs = runtimelib::list_kernelspecs().await;
    // the deno kernel
    let kernel_specification = kernelspecs.get(0).unwrap();

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

    let working_directory = "wkdir";
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

    // Listen for display data, execute result, stdout messages, etc.
    let mut iopub_socket =
        runtimelib::create_client_iopub_connection(&connection_info, "", &session_id).await?;
    let mut shell_socket =
        runtimelib::create_client_shell_connection(&connection_info, &session_id).await?;
    // Control socket is for kernel management, not used here
    // let mut control_socket =
    //     runtimelib::create_client_control_connection(&connection_info, &session_id).await?;

    let execute_request = ExecuteRequest::new("console.log('Hello, World!')".to_string());
    let execute_request: JupyterMessage = execute_request.into();
    let execute_request_id = execute_request.header.msg_id.clone();

    let iopub_handle = tokio::spawn({
        async move {
            loop {
                match iopub_socket.read().await {
                    Ok(message) => match message.content {
                        JupyterMessageContent::Status(status) => {
                            //
                            if status.execution_state == ExecutionState::Idle
                                && message.parent_header.as_ref().map(|h| h.msg_id.as_str())
                                    == Some(execute_request_id.as_str())
                            {
                                println!("Execution finalized, exiting...");
                                break;
                            }
                        }
                        _ => {
                            println!("{:?}", message.content);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error receiving iopub message: {}", e);
                        break;
                    }
                }
            }
        }
    });

    shell_socket.send(execute_request).await?;

    iopub_handle.await?;

    process.start_kill()?;

    Ok(())
}
