# Playwright Browser version of Web store order processor example

[Web store order processor example robot](https://github.com/robocorp/example-web-store-order-processor) but implemented with Playwright based Browser library.

See https://robocorp.com/docs/development-guide/browser/playwright for an in-depth look into the library itself.

Why use Playwright based Browser library?

- Implicit retrying on Getters
- More concise API
- Execution speed through Context and Page abstractions, enabling multi-login and multi-tab automation without multiple browsers

## Configure local vault

See https://robocorp.com/docs/development-guide/variables-and-secrets/vault

Paste this content in the vault file:

```json
{
  "swaglabs": {
    "username": "standard_user",
    "password": "secret_sauce"
  }
}
```

In `devdata/env.json`, edit the `RPA_SECRET_FILE` variable to point to the
`vault.json` file on your filesystem. On macOS / Linux, use normal file paths,
for example, `"/Users/<username>/vault.json"` or `"/home/<username>/vault.json"`.
On Windows 10, you need to escape the path, for example, `"C:\\Users\\User\\vault.json"`.
