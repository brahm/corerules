// A separate file rather than an inline script, because the page's CSP is `default-src 'self'`
// and inline script would be blocked — leaving the readout blank, which is the one value this
// spike exists to show.
for (const [key, value] of Object.entries(window.spike)) {
  document.getElementById(key).textContent = value
}
