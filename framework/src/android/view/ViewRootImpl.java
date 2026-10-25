package android.view;

import android.util.AndroidRuntimeException;

public final class ViewRootImpl {
    private ViewRootImpl() {}

    public static class CalledFromWrongThreadException extends AndroidRuntimeException {
        public CalledFromWrongThreadException(String msg) {
            super(msg);
        }
    }
}
