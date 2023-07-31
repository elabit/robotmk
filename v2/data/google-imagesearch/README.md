# Google image search example robot

Executes Google image search and stores the first result image.
(works with any browser)

## Troubleshoot

If you're experiencing issues on a Mac while using Chrome, please set the following
environment variables in the [env.json](./devdata/env.json) file:

```JSON
{
    "USE_CHROME": "1",
    "RPA_SELENIUM_BINARY_LOCATION": "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
}
```

These will enforce the usage of a Chrome browser while explicitly downloading a
corresponding *chromedriver* version matching your browser. Since some webdrivers can't
detect your browser location automatically, you can set one explicitly with
`RPA_SELENIUM_BINARY_LOCATION`.

This requires `rpaframework>=24.1.0`.
