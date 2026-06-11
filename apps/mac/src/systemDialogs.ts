import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { open as openPath } from "@tauri-apps/plugin-shell";

export async function chooseVideoFile(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose Video File",
    defaultPath: defaultPath || undefined,
    filters: [{ name: "Video", extensions: ["mov", "mp4", "m4v", "webm"] }],
  });
  return singlePath(path);
}

export async function choosePngSequence(defaultPath?: string) {
  const paths = await openDialog({
    title: "Choose PNG Frames",
    defaultPath: defaultPath || undefined,
    multiple: true,
    filters: [{ name: "PNG Frames", extensions: ["png"] }],
  });
  return Array.isArray(paths) ? paths : paths ? [paths] : [];
}

export async function chooseSpriteSheetFile(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose Sprite Sheet",
    defaultPath: defaultPath || undefined,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }],
  });
  return singlePath(path);
}

export async function chooseGsfpackFolder(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose .gsfpack Folder",
    defaultPath: defaultPath || undefined,
    directory: true,
    recursive: true,
  });
  return singlePath(path);
}

export async function chooseOutputFolder(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose Output Folder",
    defaultPath: defaultPath || undefined,
    directory: true,
    canCreateDirectories: true,
  });
  return singlePath(path);
}

export async function chooseFfmpegBinary(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose ffmpeg Binary",
    defaultPath: defaultPath || undefined,
  });
  return singlePath(path);
}

export async function chooseFfprobeBinary(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose ffprobe Binary",
    defaultPath: defaultPath || undefined,
  });
  return singlePath(path);
}

export async function openFileOrFolder(path: string) {
  await openPath(path);
}

function singlePath(value: string | string[] | null) {
  return Array.isArray(value) ? value[0] ?? null : value;
}
