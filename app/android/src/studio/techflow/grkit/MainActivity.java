package studio.techflow.grkit;

import android.app.Activity;
import android.content.Context;
import android.content.res.Configuration;
import android.content.pm.ApplicationInfo;
import android.os.Bundle;
import com.longbridge.nsh.GrkitView;

public final class MainActivity extends Activity {
    /**
     * The system's views here, such as the tab bar with its fixed height,
     * clip larger text. The app's own text follows the system through
     * {@link GrkitView#setFontScale} and has its own limit.
     */
    private static final float MAX_NATIVE_FONT_SCALE = 1.15f;

    private GrkitView view;
    private static native void nativeInitialize(String cameraUrl, String releaseBase, String demoScreen, String releasePublicKey);

    @Override
    protected void attachBaseContext(Context base) {
        super.attachBaseContext(base);
        if (base.getResources().getConfiguration().fontScale > MAX_NATIVE_FONT_SCALE) {
            Configuration limit = new Configuration();
            limit.fontScale = MAX_NATIVE_FONT_SCALE;
            applyOverrideConfiguration(limit);
        }
    }

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        System.loadLibrary("nsh");
        boolean development = (getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0;
        // A development build also takes the demo screen from the launch
        // intent (`adb shell am start ... --es demo_screen A5`).
        String demoScreen = development ? getIntent().getStringExtra("demo_screen") : null;
        if (demoScreen == null) demoScreen = BuildConfig.DEMO_SCREEN;
        nativeInitialize(BuildConfig.CAMERA_URL, BuildConfig.RELEASE_BASE, demoScreen,
                development ? BuildConfig.RELEASE_PUBLIC_KEY : "");
        CameraBridge.initialize(this);
        view = new GrkitView(this);
        view.registerComponent(ActivityIndicator.NAME, context -> new ActivityIndicator(this));
        view.setFontScale(systemFontScale());
        setContentView(view);
        view.runRustApp("nsh");
    }

    @Override
    protected void onResume() {
        super.onResume();
        CameraBridge.onResume();
    }

    @Override
    protected void onPause() {
        CameraBridge.onPause();
        super.onPause();
    }

    @Override
    public void onConfigurationChanged(Configuration configuration) {
        super.onConfigurationChanged(configuration);
        GrkitView.updateSystemBars(this);
        if (view != null) view.setFontScale(systemFontScale());
    }

    /** The person's text size: the activity's own configuration is limited. */
    private float systemFontScale() {
        return getApplicationContext().getResources().getConfiguration().fontScale;
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(requestCode, permissions, results);
        CameraBridge.onRequestPermissionsResult(requestCode, permissions, results);
    }

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        if (view == null || !view.handleBack()) super.onBackPressed();
    }

    @Override
    protected void onDestroy() {
        if (view != null) view.destroy();
        CameraBridge.shutdown();
        super.onDestroy();
    }
}
