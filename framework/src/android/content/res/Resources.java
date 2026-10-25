package android.content.res;

import android.util.DisplayMetrics;
import android.winrt.Host;
import android.winrt.Native;

public class Resources {
    public static class NotFoundException extends RuntimeException {
        public NotFoundException(String message) {
            super(message);
        }
    }

    private static Resources sInstance;
    private final DisplayMetrics mMetrics = new DisplayMetrics();

    public static Resources winrtInstance() {
        if (sInstance == null) {
            sInstance = new Resources();
        }
        return sInstance;
    }

    private Resources() {}

    public DisplayMetrics getDisplayMetrics() {
        float d = Host.DENSITY;
        mMetrics.density = d;
        mMetrics.scaledDensity = d;
        mMetrics.densityDpi = (int) (d * 160f);
        mMetrics.widthPixels = Host.sWidth;
        mMetrics.heightPixels = Host.sHeight;
        return mMetrics;
    }

    public String getString(int id) {
        String s = Native.resString(id);
        if (s == null) {
            throw new NotFoundException("String resource not found");
        }
        return s;
    }

    public String getString(int id, Object... args) {
        return format(getString(id), args);
    }

    public CharSequence getText(int id) {
        return getString(id);
    }

    public int getColor(int id) {
        int t = Native.resType(id);
        if (t < 0x1c || t > 0x1f) {
            throw new NotFoundException("Color resource not found");
        }
        return colorOf(t, Native.resData(id));
    }

    public float getDimension(int id) {
        int t = Native.resType(id);
        if (t != 5) {
            throw new NotFoundException("Dimension resource not found");
        }
        return complexToPx(Native.resData(id), Host.DENSITY);
    }

    public int getDimensionPixelSize(int id) {
        return roundPx(getDimension(id));
    }

    public int getDimensionPixelOffset(int id) {
        return (int) getDimension(id);
    }

    public int getInteger(int id) {
        int t = Native.resType(id);
        if (t < 0x10 || t > 0x1f) {
            throw new NotFoundException("Integer resource not found");
        }
        return Native.resData(id);
    }

    public boolean getBoolean(int id) {
        int t = Native.resType(id);
        if (t < 0x10 || t > 0x1f) {
            throw new NotFoundException("Boolean resource not found");
        }
        return Native.resData(id) != 0;
    }

    public int getIdentifier(String name, String defType, String defPackage) {
        return Native.resId(name, defType);
    }

    public static int roundPx(float f) {
        int r = (int) (f + (f < 0f ? -0.5f : 0.5f));
        if (r == 0 && f > 0f) {
            return 1;
        }
        return r;
    }

    public static int colorOf(int type, int data) {
        if (type == 0x1d) {
            return data | 0xFF000000;
        }
        if (type == 0x1e || type == 0x1f) {
            int a = type == 0x1e ? (data >> 12) & 0xF : 0xF;
            int r = (data >> 8) & 0xF;
            int g = (data >> 4) & 0xF;
            int b = data & 0xF;
            return ((a * 17) << 24) | ((r * 17) << 16) | ((g * 17) << 8) | (b * 17);
        }
        return data;
    }

    public static float complexToFloat(int c) {
        int r = (c >> 4) & 3;
        float mult = r == 0 ? 1f / 256f : r == 1 ? 1f / 32768f : r == 2 ? 1f / 8388608f : 1f / 2147483648f;
        return (float) (c & 0xFFFFFF00) * mult;
    }

    public static int complexUnit(int c) {
        return c & 0xF;
    }

    public static float complexToPx(int c, float density) {
        float v = complexToFloat(c);
        int u = c & 0xF;
        if (u == 1 || u == 2) {
            return v * density;
        }
        if (u == 3) {
            return v * density * 160f / 72f;
        }
        if (u == 4) {
            return v * density * 160f;
        }
        if (u == 5) {
            return v * density * 160f / 25.4f;
        }
        return v;
    }

    public static String format(String fmt, Object[] args) {
        return Native.format(fmt, args);
    }
}
