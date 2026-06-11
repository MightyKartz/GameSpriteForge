import { useEffect, useState } from "react";
import { readPreviewImage } from "../tauriCommands";

export function usePreviewImage(path: string | null | undefined, fallbackSrc: string | null = null) {
  const [src, setSrc] = useState<string | null>(fallbackSrc);

  useEffect(() => {
    let cancelled = false;
    if (!path) {
      setSrc(fallbackSrc);
      return () => {
        cancelled = true;
      };
    }

    setSrc(fallbackSrc);
    void readPreviewImage(path)
      .then((dataUrl) => {
        if (!cancelled) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSrc(fallbackSrc);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [fallbackSrc, path]);

  return src;
}
