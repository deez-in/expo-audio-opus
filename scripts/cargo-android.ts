import path from "path";
import fs from "fs";
import { spawnSync } from "child_process";

const TARGET_TO_DESTINATION = {
  "aarch64-linux-android": "arm64-v8a",
  "armv7-linux-androideabi": "armeabi-v7a",
  "x86_64-linux-android": "x86_64",
} as const;

function build(target: string) {
  console.log(`Compiling for target ${target}...`);
  const result = spawnSync(
    "cargo",
    ["ndk", "--target", target, "--platform", "26", "build", "--release", "--features=jni"],
    {
      stdio: "inherit",
    }
  );
  if (result.status !== 0) {
    console.warn(`cargo-ndk build failed for target ${target}, exit code: ${result.status}`);
  }
}

function main() {
  console.log("Building rust library for android");

  process.chdir("opus-pure");

  Object.keys(TARGET_TO_DESTINATION).forEach(build);

  process.chdir("..");

  Object.entries(TARGET_TO_DESTINATION).forEach(([target, architecture]) => {
    const sourcePath = path.join(
      process.cwd(),
      "opus-pure",
      "target",
      target,
      "release",
      "libexpo_audio_opus.so"
    );
    const architecturePath = path.join(
      process.cwd(),
      "android",
      "src",
      "main",
      "jniLibs",
      architecture
    );
    if (!fs.existsSync(architecturePath)) {
      fs.mkdirSync(architecturePath, { recursive: true });
    }
    if (fs.existsSync(sourcePath)) {
      fs.copyFileSync(
        sourcePath,
        path.join(architecturePath, "libexpo_audio_opus.so")
      );
      console.log(`Copied ${architecture} shared library to ${architecturePath}`);
    }
  });
}

main();
