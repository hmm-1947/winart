package android.view;

import android.content.Context;
import android.winrt.Native;
import android.winrt.XmlAttrs;

public class LayoutInflater {
    private static final int END_DOCUMENT = 1;
    private static final int START_TAG = 2;
    private static final int END_TAG = 3;

    private final Context mContext;

    public LayoutInflater(Context context) {
        mContext = context;
    }

    public static LayoutInflater from(Context context) {
        return new LayoutInflater(context);
    }

    public Context getContext() {
        return mContext;
    }

    public View inflate(int resource, ViewGroup root) {
        return inflate(resource, root, root != null);
    }

    public View inflate(int resource, ViewGroup root, boolean attachToRoot) {
        int h = Native.xmlOpen(resource);
        if (h < 0) {
            throw new InflateException("Layout resource not found");
        }
        try {
            int ev = Native.xmlNext(h);
            while (ev != START_TAG) {
                if (ev == END_DOCUMENT) {
                    throw new InflateException("No start tag found");
                }
                ev = Native.xmlNext(h);
            }
            String name = Native.xmlName(h);
            XmlAttrs attrs = new XmlAttrs(h);
            if (name.equals("merge")) {
                if (root == null || !attachToRoot) {
                    throw new InflateException("merge needs a root to attach to");
                }
                rInflate(h, root);
                return root;
            }
            View result = createViewFromTag(name, attrs);
            ViewGroup.LayoutParams params = null;
            if (root != null) {
                params = root.generateLayoutParams(attrs);
                if (!attachToRoot) {
                    result.setLayoutParams(params);
                }
            }
            if (result instanceof ViewGroup) {
                rInflate(h, (ViewGroup) result);
            }
            if (root != null && attachToRoot) {
                root.addView(result, params);
                return root;
            }
            return result;
        } finally {
            Native.xmlClose(h);
        }
    }

    private void rInflate(int h, ViewGroup parent) {
        while (true) {
            int ev = Native.xmlNext(h);
            if (ev == END_DOCUMENT || ev == END_TAG) {
                return;
            }
            if (ev != START_TAG) {
                continue;
            }
            String name = Native.xmlName(h);
            XmlAttrs attrs = new XmlAttrs(h);
            if (name.equals("include")) {
                int layout = attrs.getAttributeResourceValue(null, "layout", 0);
                int wi = attrs.indexOf(XmlAttrs.NS, "layout_width");
                int hi = attrs.indexOf(XmlAttrs.NS, "layout_height");
                int incW = wi >= 0 ? attrs.dimenPx(wi, ViewGroup.LayoutParams.WRAP_CONTENT) : 0;
                int incH = hi >= 0 ? attrs.dimenPx(hi, ViewGroup.LayoutParams.WRAP_CONTENT) : 0;
                int idIndex = attrs.indexOf(XmlAttrs.NS, "id");
                int incId = idIndex >= 0 ? attrs.refId(idIndex) : View.NO_ID;
                skip(h);
                if (layout == 0) {
                    throw new InflateException("include needs a layout attribute");
                }
                View v = inflate(layout, parent, false);
                ViewGroup.LayoutParams lp = v.getLayoutParams();
                if (wi >= 0) {
                    lp.width = incW;
                }
                if (hi >= 0) {
                    lp.height = incH;
                }
                if (incId != View.NO_ID) {
                    v.setId(incId);
                }
                parent.addView(v, lp);
                continue;
            }
            if (name.equals("requestFocus") || name.equals("tag")) {
                skip(h);
                continue;
            }
            View v = createViewFromTag(name, attrs);
            ViewGroup.LayoutParams lp = parent.generateLayoutParams(attrs);
            if (v instanceof ViewGroup) {
                rInflate(h, (ViewGroup) v);
            } else {
                skip(h);
            }
            parent.addView(v, lp);
        }
    }

    private static void skip(int h) {
        int depth = 1;
        while (depth > 0) {
            int ev = Native.xmlNext(h);
            if (ev == START_TAG) {
                depth++;
            } else if (ev == END_TAG) {
                depth--;
            } else if (ev == END_DOCUMENT) {
                return;
            }
        }
    }

    private View createViewFromTag(String name, XmlAttrs attrs) {
        Object o = Native.createView(name, mContext, attrs);
        if (o == null) {
            throw new InflateException("Error inflating class");
        }
        return (View) o;
    }
}
