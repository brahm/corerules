const { app, BrowserWindow } = require('electron')
const path = require('node:path')

// The window exists only so that "did it launch?" has a visible answer. It reports the
// versions and architecture, because ticket 12's question is architecture-specific: Apple
// Silicon refuses unsigned arm64 code, while translated x86_64 runs unsigned.
function createWindow() {
  const win = new BrowserWindow({
    width: 620,
    height: 420,
    title: 'corerules spike',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js')
    }
  })
  win.loadFile(path.join(__dirname, 'index.html'))
}

app.whenReady().then(() => {
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
