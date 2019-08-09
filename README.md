### **A Simple PSN API wrapper**

#### This is a working in progress. It have very limit API functions for now.

#### Usage:
`Used to communicate with PSN network API.`<br>

#### Features:
*. This crate use actix_web_client as http client connecting to PSN network.<br>
*. If you want to use another http client. You could add `features = ["no-client"]` in `cargo.toml` and use the trait crate provide and impl it to your preferred http client<br>


