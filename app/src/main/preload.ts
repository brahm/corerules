/**
 * The bridge, and it is deliberately dull.
 *
 * Every function here is one `invoke` and nothing else. A preload that computed anything
 * would be logic living in the one place that is neither testable like the engine nor
 * inspectable like the renderer.
 */
import { contextBridge, ipcRenderer } from "electron";
import { CHANNEL } from "./api.ts";

contextBridge.exposeInMainWorld("corerules", {
  root: () => ipcRenderer.invoke(CHANNEL.root),
  chooseRoot: () => ipcRenderer.invoke(CHANNEL.chooseRoot),
  packs: () => ipcRenderer.invoke(CHANNEL.packs),
  characters: () => ipcRenderer.invoke(CHANNEL.characters),
  open: (id: string) => ipcRenderer.invoke(CHANNEL.open, id),
  steps: (packId: string, draft: unknown) => ipcRenderer.invoke(CHANNEL.steps, packId, draft),
  create: (packId: string, draft: unknown, hitDie: number) =>
    ipcRenderer.invoke(CHANNEL.create, packId, draft, hitDie),
});
