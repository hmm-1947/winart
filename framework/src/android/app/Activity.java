package android.app;

import android.content.ContextWrapper;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.LayoutInflater;
import android.view.View;

import android.winrt.Native;

public class Activity extends ContextWrapper {
    private View mContent;
    private boolean mFinished;

    public Activity() {}

    protected void onCreate(Bundle savedInstanceState) {}

    protected void onStart() {}

    protected void onResume() {}

    protected void onPause() {}

    protected void onStop() {}

    protected void onDestroy() {}

    public void setContentView(View v) {
        mContent = v;
        Native.invalidate();
    }

    public void setContentView(int layoutResID) {
        setContentView(LayoutInflater.from(this).inflate(layoutResID, null));
    }

    public LayoutInflater getLayoutInflater() {
        return LayoutInflater.from(this);
    }

    public View findViewById(int id) {

        return mContent == null ? null : mContent.findViewById(id);
    }

    public void setTitle(CharSequence t) {
        Native.setTitle(t == null ? "" : t.toString());
    }

    public void finish() {
        if (!Native.isMainThread()) {
            new Handler(Looper.getMainLooper()).post(new Runnable() {
                public void run() {
                    finish();
                }
            });
            return;
        }
        mFinished = true;
        Native.exit();
    }

    public boolean isFinishing() {
        return mFinished;
    }

    public final void runOnUiThread(Runnable action) {
        if (Native.isMainThread()) {
            action.run();
        } else {
            new Handler(Looper.getMainLooper()).post(action);
        }
    }

    public final View winrtContent() {
        return mContent;
    }

    public final void winrtPerformCreate() {
        onCreate(null);
        onStart();
        onResume();
    }
}
