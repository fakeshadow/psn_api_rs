use std::sync::{Arc, Mutex};
use std::time::Duration;

use psn_api_rs::{models::TrophySet, psn::PSN, traits::PSNRequest};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let refresh_token = "refresh_token";
    let npsso = "npsso";

    let mut psn = PSN::new()
        .set_region("us".to_owned())
        .set_lang("en".to_owned())
        .set_self_online_id(String::from("Your Login account PSN online_id"))
        .add_refresh_token(refresh_token.into())
        .add_npsso(npsso.into());

    // the max proxy pool size is determined by the first proxies vector's length passed to PSN object.
    // You can pass more proxies on the fly to PSN but once you hit the max pool size
    // all additional proxies become backup and can only be activated when an active proxy is dropped(connection broken for example)
    let proxies = vec![
        // ("address", Some(username), Some(password)),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
        ("http://10.0.0.10:3128", None, None),
    ];

    psn.add_proxy(proxies).await;

    psn = psn.auth().await.unwrap_or_else(|e| panic!("{:?}", e));

    println!(
        "\r\nAuthentication Success! You PSN object is:\r\n{:#?}",
        psn
    );

    let psn = Arc::new(psn);
    let result = Arc::new(Mutex::new(Vec::new()));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<u8>(100);

    for i in 0..20 {
        let psn = psn.clone();
        let result = result.clone();
        let mut tx = tx.clone();
        tokio::spawn(async move {
            println!("remain proxy clients in pool: {:?}", psn.clients_state());
            let set = psn
                .get_trophy_set::<TrophySet>("Hakoom", "NPWR10788_00")
                .await
                .unwrap_or_else(|e| panic!("{:?}", e));
            result.lock().unwrap().push(set);
            let _ = tx.send(i).await;
        });
    }

    drop(tx);

    while let Some(index) = rx.recv().await {
        println!("job {} is done", index);
    }

    println!("total result count {:?}", result.lock().unwrap().len());

    Ok(())
}
