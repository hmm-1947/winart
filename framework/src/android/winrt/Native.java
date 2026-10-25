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

    public static native int resType(int id);

    public static native int resData(int id);

    public static native String resString(int id);

    public static native int resId(String name, String type);

    public static native boolean classExists(String name);

    public static native String format(String fmt, Object[] args);

    public static native void dumpStr(String s);

    public static native void dumpInt(int v);

    public static native void dumpHex(int v);

    public static native String dumpTake();

    public static native Object createView(String cls, Object ctx, Object attrs);

    public static native boolean invokeClick(Object target, String method, Object view);

    public static native int xmlOpen(int id);

    public static native int xmlNext(int h);

    public static native void xmlClose(int h);

    public static native String xmlName(int h);

    public static native int xmlAttrCount(int h);

    public static native String xmlAttrNs(int h, int i);

    public static native String xmlAttrName(int h, int i);

    public static native int xmlAttrRes(int h, int i);

    public static native int xmlAttrType(int h, int i);

    public static native int xmlAttrData(int h, int i);

    public static native String xmlAttrString(int h, int i);
}
