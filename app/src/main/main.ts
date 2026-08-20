/**
 * The main process. It owns the file system, the Library and every pack; the renderer owns
 * the window and nothing else.
 *
 * §8 splits storage by who owns the data, and this is that split made concrete: the **content
 * root** is a folder the user picks — visible, because backup is their job by design — while
 * the one thing this process remembers about itself, which folder that was, goes to the OS
 * convention path, precisely because it must not travel with the content.
 */
import { app, BrowserWindow, dialog, ipcMain } from "electron";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Library } from "../../../engine/src/library.ts";
import { CHANNEL } from "./api.ts";
import * as service from "./service.ts";

const here = dirname(fileURLToPath(import.meta.url));

/** §8: the first-run default must be VISIBLE on all three systems. */
function defaultRoot(): string {
  return process.platform === "linux"
    ? join(homedir(), "corerules")
    : join(app.getPath("documents"), "corerules");
}

/** Application state, at the OS convention path, because it must not travel. */
function settingsPath(): string {
  return join(app.getPath("userData"), "settings.json");
}

function readRoot(): string {
  // An override, because a support conversation that begins "open the app and click through
  // to the folder" is worse than one that begins "run it with CORERULES_ROOT set".
  const override = process.env["CORERULES_ROOT"];
  if (override !== undefined && override !== "") return override;
  try {
    return (JSON.parse(readFileSync(settingsPath(), "utf8")) as { root?: string }).root ?? defaultRoot();
  } catch {
    return defaultRoot();
  }
}

function writeRoot(root: string): void {
  mkdirSync(dirname(settingsPath()), { recursive: true });
  writeFileSync(settingsPath(), `${JSON.stringify({ root }, null, 2)}\n`);
}

let root = "";
const library = (): Library => new Library(root);

function handlers(): void {
  ipcMain.handle(CHANNEL.root, () => root);

  ipcMain.handle(CHANNEL.chooseRoot, async () => {
    const picked = await dialog.showOpenDialog({
      title: "Where your packs and characters live",
      defaultPath: existsSync(root) ? root : homedir(),
      properties: ["openDirectory", "createDirectory"],
    });
    if (picked.canceled || picked.filePaths[0] === undefined) return undefined;
    root = picked.filePaths[0];
    writeRoot(root);
    return root;
  });

  ipcMain.handle(CHANNEL.packs, () => service.packs(library()));
  ipcMain.handle(CHANNEL.characters, () => service.characters(library()));
  ipcMain.handle(CHANNEL.open, (_event, id: string) => service.open(library(), id));
  ipcMain.handle(CHANNEL.timeline, (_e, id: string) => service.timeline(library(), id));
  ipcMain.handle(CHANNEL.levelUp, (_e, id: string, classId: string, die: number, chose) =>
    service.levelUp(library(), id, classId, die, chose));
  ipcMain.handle(CHANNEL.correctEvent, (_e, id: string, eventId: string, replacement) =>
    service.correctEvent(library(), id, eventId, replacement));
  ipcMain.handle(CHANNEL.removeEvent, (_e, id: string, eventId: string) =>
    service.removeEvent(library(), id, eventId));
  ipcMain.handle(CHANNEL.arms, (_e, packId: string, classId: string) =>
    service.arms(library(), packId, classId));
  ipcMain.handle(CHANNEL.wear, (_e, id: string, worn: string[]) =>
    service.wear(library(), id, worn));
  ipcMain.handle(CHANNEL.steps, (_e, packId: string, draft) => service.steps(library(), packId, draft));
  ipcMain.handle(CHANNEL.create, (_e, packId: string, draft, hitDie: number) =>
    service.create(library(), packId, draft, hitDie));
}

function window(): void {
  const win = new BrowserWindow({
    width: 1100,
    height: 800,
    title: "corerules",
    backgroundColor: "#12100e",
    webPreferences: {
      preload: join(here, "preload.cjs"),
      // The three that matter, and they are not defaults to be lazy about: the renderer
      // holds no Node, shares no context with the preload, and runs sandboxed. What it can
      // do is exactly what `api.ts` lists.
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });
  void win.loadFile(join(here, "index.html"));

  // A way to see what the window renders on a machine with no display: load it, read the
  // text back out, print it and quit. Guarded by an environment variable because it is a
  // smoke path and not a feature — but it is the only thing that proves the two processes
  // actually talk, which no unit test can.
  if (process.env["CORERULES_SMOKE"] !== undefined) {
    win.webContents.once("did-finish-load", () => {
      setTimeout(() => {
        // Click the first character, so the smoke path reaches the sheet — which is the
        // screen this whole project is for.
        const steps: Record<string, string> = {
          sheet: "document.querySelector('.link')?.click(); await new Promise(r => setTimeout(r, 500));",
          correct: `
            document.querySelector('.link')?.click();
            await new Promise(r => setTimeout(r, 500));
            const die = [...document.querySelectorAll('.value input[type=number]')][0];
            die.value = '3';
            // React maps onBlur to focusout, because blur does not bubble. Dispatching a
            // raw 'blur' here fired nothing and made the die edit look broken when it was not.
            die.dispatchEvent(new Event('focusout', { bubbles: true }));
            await new Promise(r => setTimeout(r, 600));
            const sel = document.querySelector('.value select');
            sel.value = 'phb:mage';
            sel.dispatchEvent(new Event('change', { bubbles: true }));
            await new Promise(r => setTimeout(r, 700));
          `,
          advance: `
            document.querySelector('.link')?.click();
            await new Promise(r => setTimeout(r, 500));
            [...document.querySelectorAll('button')].find(b => /^Advance /.test(b.textContent)).click();
            await new Promise(r => setTimeout(r, 700));
          `,
          create: `
            [...document.querySelectorAll('button')].find(b => b.textContent === 'Create a character').click();
            await new Promise(r => setTimeout(r, 400));
            document.querySelector('.name').value = 'Balin';
            document.querySelector('.name').dispatchEvent(new Event('input', { bubbles: true }));
            [...document.querySelectorAll('button')].find(b => /Roll them/.test(b.textContent)).click();
            await new Promise(r => setTimeout(r, 400));
            [...document.querySelectorAll('.offer')].find(b => b.textContent.startsWith('Dwarf')).click();
            await new Promise(r => setTimeout(r, 400));
            [...document.querySelectorAll('.offer')].find(b => b.textContent.startsWith('Fighter')).click();
            await new Promise(r => setTimeout(r, 500));
          `,
        };
        const click = steps[process.env["CORERULES_SMOKE"] ?? ""] ?? "";
        void win.webContents.executeJavaScript(`(async () => { ${click} return document.body.innerText; })()`)
          .then((text: string) => { console.log(text); })
          .finally(() => { app.quit(); });
      }, 400);
    });
  }
}

void app.whenReady().then(() => {
  root = readRoot();
  handlers();
  window();
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) window();
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});
