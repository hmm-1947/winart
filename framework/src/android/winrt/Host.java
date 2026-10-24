package android.winrt;

import android.app.Activity;
import android.view.View;

public final class Host {
    public static float DENSITY = 2.0f;
    private static Activity sActivity;
    private static View sTarget;

    private Host() {}

    public static void init(float density) {
        DENSITY = density;
    }

    public static boolean clickByText(String s) {
        View r = sActivity == null ? null : sActivity.winrtContent();
        return r != null && clickIn(r, s);
    }

    private static boolean clickIn(View v, String s) {
        if (v instanceof android.widget.Button) {
            if (((android.widget.Button) v).getText().toString().equals(s)) {
                return v.performClick();
            }
            return false;
        }
        if (v instanceof android.view.ViewGroup) {
            android.view.ViewGroup g = (android.view.ViewGroup) v;
            for (int i = 0; i < g.getChildCount(); i++) {
                if (clickIn(g.getChildAt(i), s)) {
                    return true;
                }
            }
        }
        return false;
    }

    public static void start(Activity a, String pkg) {
        sActivity = a;
        a.winrtSetPackage(pkg);
        a.winrtPerformCreate();
    }

    public static void draw(int w, int h) {
        Native.fillRect(0, 0, w, h, 0xFFFFFFFF);
        View r = sActivity == null ? null : sActivity.winrtContent();
        if (r == null) {
            return;
        }
        r.measure(View.MeasureSpec.makeMeasureSpec(w, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(h, View.MeasureSpec.EXACTLY));
        r.layout(0, 0, w, h);
        View.winrtUpdateAbs(r);
        View.winrtDrawTree(r);
    }

    public static void touch(int action, int x, int y) {
        View r = sActivity == null ? null : sActivity.winrtContent();
        if (r == null) {
            return;
        }
        if (action == 0) {
            sTarget = View.winrtFindTarget(r, x, y);
            if (sTarget != null) {
                View.winrtSetPressed(sTarget, true);
                Native.invalidate();
            }
        } else if (action == 1 && sTarget != null) {
            View t = sTarget;
            sTarget = null;
            View.winrtSetPressed(t, false);
            if (View.winrtContains(t, x, y)) {
                t.performClick();
            }
            Native.invalidate();
        }
    }
}
