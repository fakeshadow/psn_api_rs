use std::sync::{Arc, Mutex};

use psn_api_rs::{models::TrophySet, psn::PSN, traits::PSNRequest, types::PSNInner};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // The more tokens the higher concurrency will we be able to achieve.
    // Please note that your refresh_tokens listed here would become invalid after running the example.
    // We force generate new refresh_tokens whenever we do PSN authentication and update access_tokens.
    let refresh_tokens = vec![
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
        // "your_refresh_token",
    ];

    // temporary http client for authentication.
    let client = PSN::new_client().expect("Failed to build http client");

    let mut inners = Vec::new();

    // build PSNInner with refresh_tokens
    for refresh_token in refresh_tokens.into_iter() {
        let mut inner = PSNInner::new();

        inner
            .set_region("us".to_owned())
            .set_lang("en".to_owned())
            .set_self_online_id(String::from("Your Login account PSN online_id"))
            // multiple npsso code will also work.
            .add_refresh_token(refresh_token.into());

        inner = inner
            .auth(client.clone())
            .await
            .unwrap_or_else(|e| panic!("{:?}", e));

        inners.push(inner);
    }

    // temporary http client can be dropped when we have all PSNInners ready
    drop(client);

    let mut psn = PSN::new(inners).await;

    // proxies are not required by any means but nice to use when you want a REALLY high concurrency(like hundreds of PSNInners all with different tokens).
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
    ];

    psn = psn.init_proxy(proxies).await;

    let result = Arc::new(Mutex::new(Vec::new()));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<i32>(1000);

    // spawn 100 request to PSN at the same time.
    for i in 0..100 {
        let psn = psn.clone();
        let result = result.clone();
        let mut tx = tx.clone();
        tokio::spawn(async move {
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

    // You will keep getting all the result without triggering rate limit if you have enough PSNInners passed to PSN::new().
    println!("total result count {:?}", result.lock().unwrap().len());

    // here we just extract all the latest refresh tokens that are in pool.
    // (There could be PSNInners not returned to pool from other async tasks
    // so in real life usage you would want to check if idle_connections === connections) and sleep wait accordingly
    let inners = psn.get_inner();
    let mut tokens = Vec::new();
    let state = inners.state();
    for _i in 0..state.idle_connections {
        if let Some(inner) = inners.get().await.ok() {
            let refresh_token = inner.get_refresh_token().map(String::from);
            tokens.push(refresh_token);
        }
    }
    println!("Your new refresh_tokens are: {:#?}", tokens);

    Ok(())
}
