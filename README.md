### **A Simple PSN API wrapper**

##### This branch use `std::futures::Future` and `futures-preview` crate that support async/await syntax and only compile on the nightly rust.

<br>

#### Usage:
add 
`psn_api_rs = { git = "https://github.com/fakeshadow/psn_api_rs.git", branch = "std-future", features= [ "client" ] }`  to `cargo.toml`.<br>  
remove `features = ["client"]` if you don't want to use build in http client.(See more in documentation)

<br>

#### Features:
Use hyper::Client as http client connecting to PSN network.<br>
Get psn user profile, trophies, games info <br>
Receive/send PSN messages.<br>
Get PSN store info.

<br>

##### [Documentation](https://docs.rs/psn_api_rs/0.1.1/psn_api_rs/)


<br>
