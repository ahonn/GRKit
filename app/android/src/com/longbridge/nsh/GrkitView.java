package com.longbridge.nsh;

import android.app.Activity;

/** Give the standalone Rust app the same capabilities as NSH's packaged host. */
public final class GrkitView extends NshView {
    public GrkitView(Activity activity) {
        super(activity);
        useStandaloneLimits();
        DevActivity.edgeToEdge(activity);
    }

    public static void updateSystemBars(Activity activity) {
        DevActivity.systemBars(activity);
    }
}
