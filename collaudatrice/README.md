# Collaudatrice
This crate contains a tester for HTTP compliance and other web/webserver-related
conformance and best practices.

## Roadmap
* HTTP/1.1 tests for parsing requests and validating responses
* [Cross-Origin Resource Sharing (CORS)](https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS)
  tests.
* Analyzer for indexing the site and running various tests

### Client Support
Checks whether certain common clients understand the server. Note that browsers
that are based on these browsers (Google Chrome for Chromium, Safari for
WebKit, etc.) use basically the same transport layer (to be checked), so it
is not necessary to support these browsers separately.

* [cURL](https://curl.se/)
* [Apple WebKit](https://webkit.org)
* [Google Chromium](https://chromium.org)
* [Mozilla Firefox](https://www.mozilla.org/en-US/firefox/)

### Best Practices
* [Transport Layer Security (TLS)](https://developer.mozilla.org/en-US/docs/Glossary/TLS)
  * Obsolete protocols like SSLv2, SSLv3, TLSv1.0 and TLSv1.1 are not allowed
  * Valid certificate chain
    * Root CA included in the top stores
      * [Common CA Database (CCADB)](https://www.ccadb.org/)
      * [Mozilla Root Store](https://wiki.mozilla.org/CA)
      * [Microsoft Trusted Root Certificate Program](https://aka.ms/RootCert)
      * [Google Chrome/Chromium Root Program](https://www.chromium.org/Home/chromium-security/root-ca-policy/)
    * No expired chain certificates
    * Not self-signed
  * Supports the common TLS implementations (BoringSSL, mbed TLS, NSS, OpenSSL,
    s2n, SChannel, wolfSSL, etc.)
    * Note that it might be impossible to support certain closed-source
      implementations, notably SChannel on non-Microsoft platforms,
      Secure Transport/Network Framework on non-Apple platforms.

  * [Online Certificate Status Protocol (OCSP)](https://en.wikipedia.org/wiki/Online_Certificate_Status_Protocol) support
* OWASP best practices
