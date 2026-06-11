import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync, writeFileSync } from "node:fs";

const rootPackage = JSON.parse(readFileSync(new URL("../../package.json", import.meta.url), "utf8")) as {
  version?: string;
};

export default defineConfig({
  base: "./",
  plugins: [
    react(),
    {
      name: "forge-tauri-release-html",
      transformIndexHtml(html) {
        return html
          .replace(/\s+crossorigin(?=[\s>])/g, "")
          .replace(/<script\s+type="module"\s+src=/g, '<script defer src=');
      },
      generateBundle(_options, bundle) {
        const htmlAsset = bundle["index.html"];
        if (!htmlAsset || htmlAsset.type !== "asset" || typeof htmlAsset.source !== "string") {
          return;
        }

        let html = htmlAsset.source;
        const inlineScripts: string[] = [];
        html = html.replace(
          /<script(?:\s+type="module")?(?:\s+defer)?\s+src="\.?\/?(assets\/[^"]+\.js)"><\/script>/g,
          (match, sourcePath: string) => {
            const chunk = bundle[sourcePath];
            if (chunk?.type !== "chunk") {
              return match;
            }
            inlineScripts.push(`<script>${chunk.code}</script>`);
            return "";
          },
        );
        html = html.replace(
          /<link\s+rel="stylesheet"\s+href="\.?\/?(assets\/[^"]+\.css)">/g,
          (match, sourcePath: string) => {
            const asset = bundle[sourcePath];
            return asset?.type === "asset" && typeof asset.source === "string"
              ? `<style>${asset.source}</style>`
              : match;
          },
        );
        if (inlineScripts.length) {
          html = html.replace("</body>", `${inlineScripts.join("\n")}\n  </body>`);
        }
        htmlAsset.source = html;
      },
      closeBundle() {
        const distUrl = new URL("./dist/", import.meta.url);
        const indexUrl = new URL("index.html", distUrl);
        let html = readFileSync(indexUrl, "utf8");

        const inlineScripts: string[] = [];
        html = html.replace(
          /<script(?:\s+type="module")?(?:\s+defer)?\s+src="(\.?\/?assets\/[^"]+\.js)"><\/script>/g,
          (match, sourcePath: string) => {
            const js = readFileSync(new URL(sourcePath.replace(/^\.\//, ""), distUrl), "utf8");
            inlineScripts.push(`<script>${js.replace(/<\/script/gi, "<\\/script")}</script>`);
            return "";
          },
        );
        html = html.replace(
          /<link\s+rel="stylesheet"\s+href="(\.?\/?assets\/[^"]+\.css)">/g,
          (match, sourcePath: string) => {
            const css = readFileSync(new URL(sourcePath.replace(/^\.\//, ""), distUrl), "utf8");
            return `<style>${css.replace(/<\/style/gi, "<\\/style")}</style>`;
          },
        );
        if (inlineScripts.length) {
          html = html.replace("</body>", `${inlineScripts.join("\n")}\n  </body>`);
        }
        writeFileSync(indexUrl, html);
      }
    }
  ],
  clearScreen: false,
  define: {
    "import.meta.env.VITE_APP_VERSION": JSON.stringify(rootPackage.version ?? "0.1.0")
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2020",
    minify: false
  }
});
