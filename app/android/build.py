#!/usr/bin/env python3
"""Package the official NSH runtime with GRKit's Android camera host."""

import argparse
import json
import os
import shutil
from pathlib import Path
import subprocess
import tempfile
from urllib.parse import urlparse
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT.parent / "resources/brand/app/android"
ANDROID = ROOT / "android"


def run(*args, **kwargs):
    subprocess.run([str(arg) for arg in args], check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--abi", choices=["arm64-v8a", "armeabi-v7a", "x86_64"], default="arm64-v8a")
    parser.add_argument("--release", action="store_true", help="Disable development overrides; requires release signing environment")
    parser.add_argument("--runtime-apk", type=Path, help="Reuse a freshly built NSH APK instead of rebuilding Rust")
    parser.add_argument("--install", action="store_true")
    args = parser.parse_args()
    if args.release and "GRKIT_RELEASE_PUBLIC_KEY" in os.environ:
        raise SystemExit("GRKIT_RELEASE_PUBLIC_KEY is only allowed in development builds.")
    sdk = Path(os.environ.get("ANDROID_HOME", os.environ.get("ANDROID_SDK_ROOT", Path.home() / "Library/Android/sdk")))
    build_tools = max((sdk / "build-tools").iterdir(), key=lambda p: tuple(int(n) for n in p.name.split(".")))
    android_jar = sdk / "platforms/android-35/android.jar"
    java_home = os.environ.get("JAVA_HOME")
    if not java_home:
        raise SystemExit("Set JAVA_HOME to a JDK 17 or newer before building.")
    java_bin = Path(java_home) / "bin"
    os.environ["PATH"] = str(java_bin) + os.pathsep + os.environ["PATH"]
    out = ROOT / ".nsh/out/android"
    out.mkdir(parents=True, exist_ok=True)
    metadata = json.loads((ROOT / "nsh.json").read_text())
    runtime = args.runtime_apk
    if runtime is None:
        def package():
            run("nsh", "package", ROOT, "--target", "android", "--abi", args.abi, "--out", out / "runtime")

        # NSH fetches only from the hosts of nsh.json. A development build
        # also reaches the hosts of its overrides, such as a local release
        # fixture: nsh.json lists them while the runtime builds.
        manifest = ROOT / "nsh.json"
        listed = metadata["capabilities"]["network"]["hosts"]
        extra = [] if args.release else sorted({
            host for env in ("GRKIT_RELEASE_BASE", "GRKIT_CAMERA_URL")
            if (host := urlparse(os.environ.get(env, "")).hostname) and host not in listed})
        if extra:
            original = manifest.read_text()
            development = json.loads(original)
            development["capabilities"]["network"]["hosts"] = listed + extra
            manifest.write_text(json.dumps(development, indent=2) + "\n")
            try:
                package()
            finally:
                manifest.write_text(original)
        else:
            package()
        runtime = out / "runtime" / (metadata["name"] + ".apk")
    if args.release:
        keystore = os.environ.get("GRKIT_ANDROID_KEYSTORE")
        alias = os.environ.get("GRKIT_ANDROID_KEY_ALIAS")
        if not keystore or not alias or not os.environ.get("GRKIT_ANDROID_KEYSTORE_PASSWORD"):
            raise SystemExit("Release signing needs GRKIT_ANDROID_KEYSTORE, GRKIT_ANDROID_KEY_ALIAS and GRKIT_ANDROID_KEYSTORE_PASSWORD.")
        password = "env:GRKIT_ANDROID_KEYSTORE_PASSWORD"
    else:
        keystore = str(Path.home() / ".android/debug.keystore")
        alias, password = "androiddebugkey", "pass:android"
        if not Path(keystore).exists():
            Path(keystore).parent.mkdir(parents=True, exist_ok=True)
            run(java_bin / "keytool", "-genkeypair", "-keystore", keystore, "-storepass", "android",
                "-alias", alias, "-keypass", "android", "-dname", "CN=Android Debug,O=Android,C=US",
                "-keyalg", "RSA", "-validity", "10000")
    with tempfile.TemporaryDirectory(prefix="host-", dir=out) as temporary:
        work = Path(temporary)
        assets = work / "assets"
        with zipfile.ZipFile(runtime) as apk:
            for name in apk.namelist():
                if name.startswith("assets/"):
                    apk.extract(name, work)
        # Compile-only signatures are never included in the APK.
        stubs = work / "stubs/com/longbridge/nsh"
        stubs.mkdir(parents=True)
        (stubs / "NshView.java").write_text('''package com.longbridge.nsh;
public class NshView extends android.widget.FrameLayout {
 public NshView(android.content.Context context) { super(context); }
 void useStandaloneLimits() {}
 public void runRustApp(String path) {}
 public boolean handleBack() { return false; }
 public void destroy() {}
 public void setFontScale(float scale) {}
 public void registerComponent(String name, NshComponent.Factory factory) {}
}''')
        (stubs / "NshComponent.java").write_text('''package com.longbridge.nsh;
public interface NshComponent {
 android.view.View view();
 void setProp(String name, Object value);
 default void setAnimatedNumber(String name, double value) {}
 default void setAnimatedColor(String name, int argb) {}
 default void command(String name, Object args) {}
 default void destroy() {}
 interface Factory { NshComponent create(NshComponentContext context); }
}''')
        (stubs / "NshComponentContext.java").write_text('''package com.longbridge.nsh;
public final class NshComponentContext {}''')
        (stubs / "DevActivity.java").write_text('''package com.longbridge.nsh;
class DevActivity {
 static void edgeToEdge(android.app.Activity activity) {}
 static void systemBars(android.app.Activity activity) {}
}''')
        stub_classes = work / "stub-classes"
        stub_classes.mkdir()
        run(java_bin / "javac", "--release", "8", "-cp", android_jar, "-d", stub_classes, *stubs.glob("*.java"))
        generated = work / "BuildConfig.java"
        values = {"CAMERA_URL": "GRKIT_CAMERA_URL", "RELEASE_BASE": "GRKIT_RELEASE_BASE", "DEMO_SCREEN": "GRKIT_DEMO_SCREEN", "RELEASE_PUBLIC_KEY": "GRKIT_RELEASE_PUBLIC_KEY"}
        generated.write_text("package studio.techflow.grkit;\nfinal class BuildConfig {\n" + "".join(
            f" static final String {key} = {json.dumps('' if args.release else os.environ.get(env, ''))};\n"
            for key, env in values.items()) + "}\n")
        classes = work / "classes"
        classes.mkdir()
        run(java_bin / "javac", "--release", "8", "-cp", str(android_jar) + os.pathsep + str(stub_classes),
            "-d", classes, *sorted((ANDROID / "src").rglob("*.java")), generated)
        host_jar = work / "host.jar"
        run(java_bin / "jar", "cf", host_jar, "-C", classes, ".")
        dex = work / "dex"
        dex.mkdir()
        run(build_tools / "d8", "--min-api", "29", "--lib", android_jar, "--output", dex, host_jar)
        manifest = work / "AndroidManifest.xml"
        manifest.write_text((ANDROID / "AndroidManifest.xml").read_text().replace(
            '<application android:label=', '<application android:debuggable="' + ("false" if args.release else "true") + '" android:label='))
        resources = work / "resources.zip"
        run(build_tools / "aapt2", "compile", "--dir", ANDROID / "res", "-o", resources)
        # The launcher icon: the adaptive layers that resources/brand/build.py
        # draws (432 px: 108 dp at xxxhdpi), so the app keeps no copy of them.
        icon_res = work / "icon-res"
        (icon_res / "mipmap-xxxhdpi").mkdir(parents=True)
        (icon_res / "mipmap-anydpi").mkdir()
        for layer in ("foreground", "background", "monochrome"):
            shutil.copyfile(BRAND / f"{layer}.png", icon_res / "mipmap-xxxhdpi" / f"ic_launcher_{layer}.png")
        (icon_res / "mipmap-anydpi" / "ic_launcher.xml").write_text(
            '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
            ' <background android:drawable="@mipmap/ic_launcher_background"/>\n'
            ' <foreground android:drawable="@mipmap/ic_launcher_foreground"/>\n'
            ' <monochrome android:drawable="@mipmap/ic_launcher_monochrome"/>\n'
            '</adaptive-icon>\n')
        icons = work / "icons.zip"
        run(build_tools / "aapt2", "compile", "--dir", icon_res, "-o", icons)
        unsigned = work / "unsigned.apk"
        run(build_tools / "aapt2", "link", resources, icons, "--manifest", manifest, "-I", android_jar,
            "--min-sdk-version", "29", "--target-sdk-version", "35", "--version-code", str(metadata["build"]),
            "--version-name", metadata["version"], "-A", assets, "-o", unsigned)
        with zipfile.ZipFile(unsigned, "a") as apk, zipfile.ZipFile(runtime) as original:
            for name in original.namelist():
                if name.startswith("lib/") or name.endswith(".dex"):
                    apk.writestr(name, original.read(name), compress_type=zipfile.ZIP_STORED)
            next_dex = 2
            while f"classes{next_dex}.dex" in original.namelist():
                next_dex += 1
            apk.write(dex / "classes.dex", f"classes{next_dex}.dex", compress_type=zipfile.ZIP_STORED)
        aligned = work / "aligned.apk"
        run(build_tools / "zipalign", "-P", "16", "-f", "4", unsigned, aligned)
        final = out / "GRKit.apk"
        run(build_tools / "apksigner", "sign", "--ks", keystore, "--ks-key-alias", alias,
            "--ks-pass", password, "--out", final, aligned)
        run(build_tools / "apksigner", "verify", final)
        if args.install:
            adb = sdk / "platform-tools/adb"
            run(adb, "install", "--no-incremental", "-r", final)
            run(adb, "shell", "am", "start", "-n", "studio.techflow.grkit/.MainActivity")
        print(final)


if __name__ == "__main__":
    main()
