use anyhow::Result;

/// 再生中のNeiroへの操作
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Pause,
    Resume,
    Stop,
}

/// ## Description
/// 再生中のすべてのNeiroに操作を送る
/// ## Returns
/// 操作を送ったプレイヤーの数
#[cfg(target_os = "linux")]
pub fn send(action:Action) -> Result<usize> {
    use anyhow::{Context, bail};
    use mpris_server::zbus::{Connection, fdo::DBusProxy};

    // メディアコントロール（MPRIS）に登録されているNeiroを探して操作する
    const BUS_NAME_PREFIX: &str = "org.mpris.MediaPlayer2.neiro.";
    let method = match action {
        Action::Pause => "Pause",
        Action::Resume => "Play",
        Action::Stop => "Stop",
    };
    smol::block_on(async {
        let connection = Connection::session().await.context("Failed to connect to D-Bus")?;
        let names = DBusProxy::new(&connection).await?.list_names().await?;
        let mut count = 0;
        for name in names.iter().filter(|name| name.as_str().starts_with(BUS_NAME_PREFIX)) {
            let result = connection.call_method(
                Some(name.as_str()), "/org/mpris/MediaPlayer2", Some("org.mpris.MediaPlayer2.Player"), method, &(),
            ).await;
            // 探してから操作するまでの間に終了したプレイヤーは数えない
            if result.is_ok() {
                count += 1;
            }
        }
        if count == 0 {
            bail!("No running Neiro found");
        }
        Ok(count)
    })
}

#[cfg(not(target_os = "linux"))]
pub fn send(_action:Action) -> Result<usize> {
    anyhow::bail!("Controlling a running Neiro is only supported on Linux")
}
