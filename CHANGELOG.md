(April 26, 2020)
----------------------
### Break
- refactor some of the APIs to take agrs instead of mutate the state of `PSNInner`.
- Most API calls take `&Self`.

### Add
- experimental connection pool for http proxies.