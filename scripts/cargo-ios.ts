import path from "path";
import fs from "fs";
import { spawnSync } from "child_process";

const TARGETS = {
  ios: "aarch64-apple-ios",
  "ios-sim": "aarch64-apple-ios-sim",
};

function cargoBuild(target: string) {
  console.log(`Building for iOS target: ${target}`);
  spawnSync("cargo", ["build", "--features=ffi", "--release", "--target", target], {
    stdio: "inherit",
  });
}

function getTarget(): "ios" | "ios-sim" {
  const args = process.argv.slice(2);
  const target = (args[0] ?? "").replace("--target=", "");

  if (target !== "ios" && target !== "ios-sim") {
    console.error(
      `Invalid target ${target} found. Please specify --target='ios' or --target='ios-sim'`
    );
    process.exit(1);
  }

  return target;
}

function main() {
  const targetKey = getTarget();
  const target = TARGETS[targetKey];
  console.log(`Building ios for target ${target}`);

  process.chdir("rust");

  console.log("Building rust library for ios");
  cargoBuild(target);

  process.chdir("..");

  const destinationPath = path.join(process.cwd(), "ios", "rust");
  const rustLibPath = path.join(
    process.cwd(),
    "rust",
    "target",
    target,
    "release",
    "libexpo_audio_opus.a"
  );
  const rustHeadersPath = path.join(
    process.cwd(),
    "rust",
    "expo_audio_opus.h"
  );

  if (!fs.existsSync(destinationPath)) {
    fs.mkdirSync(destinationPath, { recursive: true });
  }
  if (fs.existsSync(rustLibPath)) {
    fs.copyFileSync(
      rustLibPath,
      path.join(destinationPath, "libexpo_audio_opus.a")
    );
  }
  if (fs.existsSync(rustHeadersPath)) {
    fs.copyFileSync(
      rustHeadersPath,
      path.join(destinationPath, "expo_audio_opus.h")
    );
  }
}

main();
