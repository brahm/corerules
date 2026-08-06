const { contextBridge } = require('electron')

// Context isolation is on by default, so the renderer cannot see `process`. Architecture is
// the value that matters here: Apple Silicon refuses unsigned arm64 code but runs translated
// x86_64 unsigned, so an x64 reading would mean Rosetta translated the app and the test
// proved nothing about the arm64 case.
contextBridge.exposeInMainWorld('spike', {
  arch: process.arch,
  platform: process.platform,
  electron: process.versions.electron,
  chrome: process.versions.chrome,
  node: process.versions.node
})
