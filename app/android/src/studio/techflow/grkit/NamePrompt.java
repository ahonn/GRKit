package studio.techflow.grkit;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.res.ColorStateList;
import android.content.res.Configuration;
import android.text.Editable;
import android.text.InputFilter;
import android.text.TextWatcher;
import android.widget.Button;
import android.text.InputType;
import android.util.TypedValue;
import android.view.WindowManager;
import android.widget.EditText;
import android.widget.FrameLayout;
import org.json.JSONObject;

/** The name alert: the system's AlertDialog with one text field. */
final class NamePrompt {
    interface Answer { void accept(String name); }

    private NamePrompt() {}

    static void show(Activity activity, JSONObject args, Answer answer) {
        if (activity == null || activity.isFinishing()) {
            answer.accept(null);
            return;
        }
        boolean night = (activity.getResources().getConfiguration().uiMode
                & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        EditText field = new EditText(activity);
        field.setSingleLine(true);
        field.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        field.setFilters(new InputFilter[] {new InputFilter.LengthFilter(args.optInt("max", 16))});
        field.setText(args.optString("value"));
        field.selectAll();
        int inset = Math.round(TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, 24,
                activity.getResources().getDisplayMetrics()));
        FrameLayout frame = new FrameLayout(activity);
        frame.setPadding(inset, 0, inset, 0);
        frame.addView(field);
        boolean[] answered = {false};
        AlertDialog dialog = new AlertDialog.Builder(activity, night
                ? android.R.style.Theme_DeviceDefault_Dialog_Alert
                : android.R.style.Theme_DeviceDefault_Light_Dialog_Alert)
                .setTitle(args.optString("title"))
                .setMessage(args.optString("message"))
                .setView(frame)
                .setNegativeButton(args.optString("cancel"), null)
                .setPositiveButton(args.optString("save"), (d, which) -> {
                    answered[0] = true;
                    answer.accept(field.getText().toString());
                })
                .create();
        dialog.setOnDismissListener(d -> {
            if (!answered[0]) answer.accept(null);
        });
        if (dialog.getWindow() != null) {
            dialog.getWindow().setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_VISIBLE);
        }
        dialog.show();
        // The app's colors, not the device theme's accent; Save is off while
        // the name is invalid or unchanged.
        int foreground = night ? 0xFFF4F4F4 : 0xFF0B0B0B;
        int disabled = night ? 0xFF6B6B6B : 0xFFA3A3A3;
        ColorStateList colors = new ColorStateList(
                new int[][] {new int[] {-android.R.attr.state_enabled}, new int[] {}},
                new int[] {disabled, foreground});
        Button save = dialog.getButton(AlertDialog.BUTTON_POSITIVE);
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setTextColor(colors);
        save.setTextColor(colors);
        field.setBackgroundTintList(ColorStateList.valueOf(foreground));
        field.setHighlightColor(night ? 0x55F4F4F4 : 0x330B0B0B);
        String original = args.optString("value").trim();
        boolean allowEmpty = args.optBoolean("allowEmpty", false);
        int max = args.optInt("max", 16);
        Runnable check = () -> save.setEnabled(
                valid(field.getText().toString().trim(), max, allowEmpty)
                        && !field.getText().toString().trim().equals(original));
        field.addTextChangedListener(new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int a, int b, int c) {}
            @Override public void onTextChanged(CharSequence s, int a, int b, int c) {}
            @Override public void afterTextChanged(Editable s) { check.run(); }
        });
        check.run();
        field.requestFocus();
    }

    /** The camera menu shows printable ASCII only (`valid_name` in Rust). */
    static boolean valid(String name, int max, boolean allowEmpty) {
        if (name.isEmpty()) return allowEmpty;
        if (name.length() > max) return false;
        for (int i = 0; i < name.length(); i++) {
            char c = name.charAt(i);
            if (c < 0x20 || c > 0x7e) return false;
        }
        return true;
    }
}
