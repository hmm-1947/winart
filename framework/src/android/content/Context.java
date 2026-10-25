package android.content;

import android.content.res.Resources;
import android.view.LayoutInflater;


public abstract class Context {
    public abstract String getPackageName();

    public Context getApplicationContext() {
        return this;
    }

    public android.os.Looper getMainLooper() {
        return android.os.Looper.getMainLooper();
    }
    public static final String LAYOUT_INFLATER_SERVICE = "layout_inflater";

    public Object getSystemService(String name) {
        if (LAYOUT_INFLATER_SERVICE.equals(name)) {
            return LayoutInflater.from(this);
        }
        return null;
    }

    public Resources getResources() {

        return Resources.winrtInstance();
    }

    public final String getString(int id) {
        return getResources().getString(id);
    }

    public final String getString(int id, Object... args) {
        return getResources().getString(id, args);
    }

    public final int getColor(int id) {
        return getResources().getColor(id);
    }
}
