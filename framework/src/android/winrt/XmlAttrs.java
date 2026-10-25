package android.winrt;

import android.content.res.Resources;
import android.util.AttributeSet;

public final class XmlAttrs implements AttributeSet {
    public static final String NS = "http://schemas.android.com/apk/res/android";

    private final int h;

    public XmlAttrs(int handle) {
        h = handle;
    }

    public int getAttributeCount() {
        return Native.xmlAttrCount(h);
    }

    public String getAttributeName(int i) {
        return Native.xmlAttrName(h, i);
    }

    public int getAttributeNameResource(int i) {
        return Native.xmlAttrRes(h, i);
    }

    public boolean isAndroid(int i) {
        String ns = Native.xmlAttrNs(h, i);
        return ns != null && ns.equals(NS);
    }

    public int indexOf(String ns, String name) {
        int n = getAttributeCount();
        for (int i = 0; i < n; i++) {
            if (name.equals(getAttributeName(i))) {
                if (ns == null) {
                    return i;
                }
                String a = Native.xmlAttrNs(h, i);
                if (a != null && a.equals(ns)) {
                    return i;
                }
            }
        }
        return -1;
    }

    public String getAttributeValue(int i) {
        return text(i);
    }

    public String getAttributeValue(String ns, String name) {
        int i = indexOf(ns, name);
        return i < 0 ? null : text(i);
    }

    public int getAttributeResourceValue(String ns, String name, int def) {
        int i = indexOf(ns, name);
        return i >= 0 && Native.xmlAttrType(h, i) == 1 ? Native.xmlAttrData(h, i) : def;
    }

    public int getAttributeIntValue(String ns, String name, int def) {
        int i = indexOf(ns, name);
        return i < 0 ? def : intVal(i, def);
    }

    public boolean getAttributeBooleanValue(String ns, String name, boolean def) {
        int i = indexOf(ns, name);
        return i < 0 ? def : bool(i, def);
    }

    public int type(int i) {
        return Native.xmlAttrType(h, i);
    }

    public int data(int i) {
        return Native.xmlAttrData(h, i);
    }

    public int rtype(int i) {
        int t = Native.xmlAttrType(h, i);
        if (t == 1) {
            return Native.resType(Native.xmlAttrData(h, i));
        }
        return t;
    }

    public int rdata(int i) {
        int t = Native.xmlAttrType(h, i);
        if (t == 1) {
            return Native.resData(Native.xmlAttrData(h, i));
        }
        return Native.xmlAttrData(h, i);
    }

    public String text(int i) {
        int t = Native.xmlAttrType(h, i);
        if (t == 1) {
            String s = Native.resString(Native.xmlAttrData(h, i));
            return s == null ? "" : s;
        }
        String raw = Native.xmlAttrString(h, i);
        return raw == null ? "" : raw;
    }

    public int refId(int i) {
        return Native.xmlAttrType(h, i) == 1 ? Native.xmlAttrData(h, i) : 0;
    }

    public int color(int i, int def) {
        int t = rtype(i);
        if (t >= 0x1c && t <= 0x1f) {
            return Resources.colorOf(t, rdata(i));
        }
        return def;
    }

    public int intVal(int i, int def) {
        int t = rtype(i);
        if (t >= 0x10 && t <= 0x1f) {
            return rdata(i);
        }
        return def;
    }

    public boolean bool(int i, boolean def) {
        int t = rtype(i);
        if (t >= 0x10 && t <= 0x1f) {
            return rdata(i) != 0;
        }
        return def;
    }

    public float floatVal(int i, float def) {
        int t = rtype(i);
        int d = rdata(i);
        if (t >= 0x10 && t <= 0x1f) {
            return (float) d;
        }
        if (t != 4) {
            return def;
        }
        boolean neg = (d >>> 31) != 0;
        int e = (d >>> 23) & 0xFF;
        int m = d & 0x7FFFFF;
        if (e == 0 && m == 0) {
            return 0f;
        }
        float v = 1f + (float) m / 8388608f;
        int x = e - 127;
        while (x > 0) {
            v = v * 2f;
            x--;
        }
        while (x < 0) {
            v = v / 2f;
            x++;
        }
        return neg ? -v : v;
    }

    public boolean isDimen(int i) {

        return rtype(i) == 5;
    }

    public float dimenValue(int i) {
        return Resources.complexToFloat(rdata(i));
    }

    public int dimenUnit(int i) {
        return Resources.complexUnit(rdata(i));
    }

    public int dimenPx(int i, int def) {
        if (rtype(i) == 5) {
            return Resources.roundPx(Resources.complexToPx(rdata(i), Host.DENSITY));
        }
        return intVal(i, def);
    }
}
