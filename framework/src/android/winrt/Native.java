package android.winrt;

public final class Native {
    private Native() {}

    public static native void fillRect(int x, int y, int w, int h, int color);

    public static native void strokeRect(int x, int y, int w, int h, int color, int width);

    public static native void drawText(String s, int x, int y, int size, int color);

    public static native int measureText(String s, int size);

    public static native void invalidate();

    public static native void setTitle(String s);

    public static native void exit();
}
