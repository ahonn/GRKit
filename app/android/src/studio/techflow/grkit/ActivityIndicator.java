package studio.techflow.grkit;

import android.content.Context;
import android.content.res.ColorStateList;
import android.content.res.Configuration;
import android.graphics.Color;
import android.view.View;
import android.widget.ProgressBar;
import com.longbridge.nsh.NshComponent;

/**
 * The platform's own loader for {@code native("grkit/activity")}: an
 * indeterminate {@link ProgressBar}, as the system and the camera app show
 * one. Props {@code light} and {@code dark}: its color in each appearance,
 * as {@code #RRGGBB}.
 */
final class ActivityIndicator implements NshComponent {
    static final String NAME = "grkit/activity";

    private final Bar bar;
    private Integer light;
    private Integer dark;

    ActivityIndicator(Context context) {
        bar = new Bar(context);
        bar.setIndeterminate(true);
    }

    @Override public View view() { return bar; }

    @Override public void setProp(String name, Object value) {
        Integer color = parse(value);
        if ("light".equals(name)) light = color;
        else if ("dark".equals(name)) dark = color;
        else return;
        apply();
    }

    private static Integer parse(Object value) {
        if (!(value instanceof String)) return null;
        try { return Color.parseColor((String) value); }
        catch (IllegalArgumentException ignored) { return null; }
    }

    private void apply() {
        boolean night = (bar.getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        Integer color = night && dark != null ? dark : light;
        if (color != null) bar.setIndeterminateTintList(ColorStateList.valueOf(color));
    }

    /** Takes its color again when the appearance changes. */
    private final class Bar extends ProgressBar {
        Bar(Context context) { super(context, null, android.R.attr.progressBarStyleSmall); }

        @Override protected void onConfigurationChanged(Configuration configuration) {
            super.onConfigurationChanged(configuration);
            apply();
        }
    }
}
