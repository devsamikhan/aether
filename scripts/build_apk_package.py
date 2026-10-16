import os
import subprocess
import zipfile
import shutil

android_jar = r'C:\Users\Latif Ullah Khan\AppData\Local\Android\Sdk\platforms\android-34\android.jar'
d8_bat = r'C:\Users\Latif Ullah Khan\AppData\Local\Android\Sdk\build-tools\34.0.0\d8.bat'
aapt_exe = r'C:\Users\Latif Ullah Khan\AppData\Local\Android\Sdk\build-tools\34.0.0\aapt.exe'
apksigner_bat = r'C:\Users\Latif Ullah Khan\AppData\Local\Android\Sdk\build-tools\34.0.0\apksigner.bat'
debug_ks = r'C:\Users\Latif Ullah Khan\.android\debug.keystore'

os.makedirs('temp_dex/com/aether/runtime', exist_ok=True)
java_code = """package com.aether.runtime;

import android.app.Activity;
import android.os.Bundle;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.graphics.Color;

public class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        WebView webView = new WebView(this);
        WebSettings settings = webView.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(true);
        settings.setAllowContentAccess(true);
        settings.setAllowUniversalAccessFromFileURLs(true);
        settings.setAllowFileAccessFromFileURLs(true);
        webView.setBackgroundColor(Color.parseColor("#070913"));
        webView.setWebViewClient(new WebViewClient());
        setContentView(webView);

        new Thread(() -> {
            boolean isLive = false;
            try {
                java.net.HttpURLConnection conn = (java.net.HttpURLConnection) new java.net.URL("http://127.0.0.1:8080/api/live-status").openConnection();
                conn.setConnectTimeout(250);
                conn.setReadTimeout(250);
                if (conn.getResponseCode() == 200) {
                    isLive = true;
                }
                conn.disconnect();
            } catch (Exception ignored) {}

            final boolean useLive = isLive;
            runOnUiThread(() -> {
                if (useLive) {
                    webView.loadUrl("http://127.0.0.1:8080/index.html");
                } else {
                    webView.loadUrl("file:///android_asset/www/index.html");
                }
            });
        }).start();
    }
}
"""
with open('temp_dex/com/aether/runtime/MainActivity.java', 'w') as f:
    f.write(java_code)

subprocess.run(['javac', '-cp', android_jar, 'temp_dex/com/aether/runtime/MainActivity.java'], check=True)
subprocess.run([d8_bat, '--lib', android_jar, 'temp_dex/com/aether/runtime/MainActivity.class', '--output', 'temp_dex'], shell=True, check=True)

manifest_xml = """<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.aether.chronopulse"
    android:versionCode="1"
    android:versionName="1.0.0">

    <uses-sdk android:minSdkVersion="21" android:targetSdkVersion="34" />
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.VIBRATE" />

    <application
        android:label="ChronoPulse"
        android:hardwareAccelerated="true"
        android:supportsRtl="true"
        android:usesCleartextTraffic="true"
        android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
        <activity
            android:name="com.aether.runtime.MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"""
with open('temp_dex/AndroidManifest.xml', 'w') as f:
    f.write(manifest_xml)

subprocess.run([aapt_exe, 'package', '-f', '-M', 'temp_dex/AndroidManifest.xml', '-I', android_jar, '-F', 'temp_dex/base.apk'], check=True)

unaligned = 'temp_dex/unaligned.apk'
final_apk = 'flagship_projects/08_aether_chronopulse_alarm/ChronoPulse.apk'

with zipfile.ZipFile('temp_dex/base.apk', 'r') as zin:
    with zipfile.ZipFile(unaligned, 'w') as zout:
        for item in zin.infolist():
            zout.writestr(item, zin.read(item.filename))
        with open('temp_dex/classes.dex', 'rb') as f:
            zout.writestr('classes.dex', f.read())
        frontend_dir = 'flagship_projects/08_aether_chronopulse_alarm/frontend'
        for root, dirs, files in os.walk(frontend_dir):
            for file in files:
                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, frontend_dir).replace('\\', '/')
                with open(full_path, 'rb') as f:
                    zout.writestr(f'assets/www/{rel_path}', f.read())

shutil.copyfile(unaligned, final_apk)
subprocess.run([
    apksigner_bat, 'sign',
    '--ks', debug_ks,
    '--ks-pass', 'pass:android',
    '--key-pass', 'pass:android',
    '--ks-key-alias', 'androiddebugkey',
    final_apk
], shell=True, check=True)

shutil.rmtree('temp_dex', ignore_errors=True)
print('Successfully rebuilt and signed ChronoPulse.apk with Live Hot-Reload Engine!')
