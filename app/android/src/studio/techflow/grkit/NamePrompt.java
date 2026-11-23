package studio.techflow.grkit;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.res.Configuration;
import android.text.InputFilter;
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
        field.requestFocus();
    }
}
