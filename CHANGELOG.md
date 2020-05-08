(May 8, 2020)
----------------------
### Break
- `refresh_token` would be updated when `PSNRequest::gen_access_and_refresh`, `PSNRequest::gen_access_from_refresh` are called and render the old `refresh_token` invalid. 
- `PSN::get_inner` has been added to give access to `Pool<PSNInnerMananger>` and it can be used to retrieve latest `refresh_token`. Examples have been updated accordingly

(April 26, 2020)
----------------------
### Break
- refactor `PSNRequest` trait and it now requires a `Client` type to pass to most of methods.
### Add
- `PSN::new()` now accept multiple `PSNInner` to achieve a high concurrency(With multiple refresh_tokens/npsso codes).

(April 26, 2020)
----------------------
### Break
- refactor some of the APIs to take agrs instead of mutate the state of `PSNInner`.
- Most API calls take `&Self`.

### Add
- experimental connection pool for http proxies.