# Example Inputs

Create the MVP green-box video fixture with:

```bash
ffmpeg -y -f lavfi -i color=c=0x00ff00:s=256x256:d=1:r=24 -vf "drawbox=x=96:y=88:w=64:h=96:color=white:t=fill" examples/inputs/green-box-character.mp4
```

Prepare deterministic manual QA fixtures for PNG sequence, sprite sheet, and safe failure-state checks with:

```bash
npm run qa:fixtures
```

The fixtures are written under `examples/inputs/manual-qa/`. They do not create or replace the real short video required by the manual QA release gate.
