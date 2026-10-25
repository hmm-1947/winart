package android.view;

import android.content.Context;
import android.os.Handler;
import android.os.Looper;
import android.util.AttributeSet;
import android.winrt.Native;
import android.winrt.Host;
import android.winrt.XmlAttrs;

public class View {
    public interface OnClickListener {
        void onClick(View v);
    }

    public static class MeasureSpec {
        public static final int MODE_SHIFT = 30;
        public static final int MODE_MASK = 3 << 30;
        public static final int UNSPECIFIED = 0;
        public static final int EXACTLY = 1 << 30;
        public static final int AT_MOST = 2 << 30;

        public static int makeMeasureSpec(int size, int mode) {
            return (size & ~MODE_MASK) | (mode & MODE_MASK);
        }

        public static int getMode(int spec) {
            return spec & MODE_MASK;
        }

        public static int getSize(int spec) {
            return spec & ~MODE_MASK;
        }
    }

    public static final int NO_ID = -1;
    public static final int VISIBLE = 0;
    public static final int INVISIBLE = 4;
    public static final int GONE = 8;

    protected Context mContext;
    ViewGroup mParent;
    ViewGroup.LayoutParams mLayoutParams;
    OnClickListener mClickListener;
    int mLeft;
    int mTop;
    int mRight;
    int mBottom;
    int mAbsX;
    int mAbsY;
    int mMeasuredWidth;
    int mMeasuredHeight;
    int mId = NO_ID;
    int mVisibility = VISIBLE;
    int mBackground;
    boolean mHasBackground;
    protected boolean mPressed;
    protected boolean mClickable;
    protected boolean mEnabled = true;
    protected int mPadLeft;
    protected int mPadTop;
    protected int mPadRight;
    protected int mPadBottom;
    protected int mMinWidth;
    protected int mMinHeight;

    public View(Context context) {
        this(context, null);
    }

    public View(Context context, AttributeSet attrs) {
        mContext = context;
        if (attrs instanceof XmlAttrs) {
            applyViewAttrs((XmlAttrs) attrs);
        }
    }

    private void applyViewAttrs(XmlAttrs a) {
        int n = a.getAttributeCount();
        int all = -1;
        int pl = -1;
        int pt = -1;
        int pr = -1;
        int pb = -1;
        for (int i = 0; i < n; i++) {
            if (!a.isAndroid(i)) {
                continue;
            }
            String name = a.getAttributeName(i);
            if (name.equals("id")) {
                mId = a.refId(i);
            } else if (name.equals("background")) {
                int t = a.rtype(i);
                if (t >= 0x1c && t <= 0x1f) {
                    mBackground = a.color(i, 0);
                    mHasBackground = true;
                }
            } else if (name.equals("padding")) {
                all = a.dimenPx(i, 0);
            } else if (name.equals("paddingLeft") || name.equals("paddingStart")) {
                pl = a.dimenPx(i, 0);
            } else if (name.equals("paddingTop")) {
                pt = a.dimenPx(i, 0);
            } else if (name.equals("paddingRight") || name.equals("paddingEnd")) {
                pr = a.dimenPx(i, 0);
            } else if (name.equals("paddingBottom")) {
                pb = a.dimenPx(i, 0);
            } else if (name.equals("minWidth")) {
                mMinWidth = a.dimenPx(i, 0);
            } else if (name.equals("minHeight")) {
                mMinHeight = a.dimenPx(i, 0);
            } else if (name.equals("visibility")) {
                int v = a.intVal(i, 0);
                mVisibility = v == 1 ? INVISIBLE : v == 2 ? GONE : VISIBLE;
            } else if (name.equals("clickable")) {
                mClickable = a.bool(i, false);
            } else if (name.equals("enabled")) {
                mEnabled = a.bool(i, true);
            } else if (name.equals("onClick")) {
                final String method = a.text(i);
                final Context ctx = mContext;
                setOnClickListener(new OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        Native.invokeClick(ctx, method, v);
                    }
                });
            }
        }
        if (all >= 0 || pl >= 0 || pt >= 0 || pr >= 0 || pb >= 0) {
            mPadLeft = pl >= 0 ? pl : all >= 0 ? all : 0;
            mPadTop = pt >= 0 ? pt : all >= 0 ? all : 0;
            mPadRight = pr >= 0 ? pr : all >= 0 ? all : 0;
            mPadBottom = pb >= 0 ? pb : all >= 0 ? all : 0;
        }
    }

    public Context getContext() {
        return mContext;
    }

    public void setId(int id) {
        mId = id;
    }

    public int getId() {
        return mId;
    }

    public View findViewById(int id) {
        return mId == id ? this : null;
    }

    public void setOnClickListener(OnClickListener l) {
        mClickListener = l;
        mClickable = true;
    }

    public boolean hasOnClickListeners() {
        return mClickListener != null;
    }

    public void setClickable(boolean c) {
        mClickable = c;
    }

    public boolean isClickable() {
        return mClickable;
    }

    public void setEnabled(boolean e) {
        mEnabled = e;
        requestLayout();
    }

    public boolean isEnabled() {
        return mEnabled;
    }

    public boolean performClick() {
        if (mClickListener != null) {
            mClickListener.onClick(this);
            return true;
        }
        return false;
    }

    public void setBackgroundColor(int color) {
        mBackground = color;
        mHasBackground = true;
        invalidate();
    }

    public void setPadding(int l, int t, int r, int b) {
        mPadLeft = l;
        mPadTop = t;
        mPadRight = r;
        mPadBottom = b;
        requestLayout();
    }

    public void setVisibility(int v) {
        mVisibility = v;
        requestLayout();
    }

    public int getVisibility() {
        return mVisibility;
    }

    public ViewGroup.LayoutParams getLayoutParams() {
        return mLayoutParams;
    }

    public void setLayoutParams(ViewGroup.LayoutParams lp) {
        mLayoutParams = lp;
        requestLayout();
    }

    public int getWidth() {
        return mRight - mLeft;
    }

    public int getHeight() {
        return mBottom - mTop;
    }

    public int getMeasuredWidth() {
        return mMeasuredWidth;
    }

    public int getMeasuredHeight() {
        return mMeasuredHeight;
    }

    public int getPaddingLeft() {
        return mPadLeft;
    }

    public int getPaddingTop() {
        return mPadTop;
    }

    public int getPaddingRight() {
        return mPadRight;
    }

    public int getPaddingBottom() {
        return mPadBottom;
    }

    private void winrtCheckThread() {
        if (!Native.isMainThread() && Host.winrtAttached(this)) {
            throw new ViewRootImpl.CalledFromWrongThreadException("Only the original thread that created a view hierarchy can touch its views.");
        }
    }

    public final View winrtParent() {
        return mParent;
    }

    private static Handler winrtMainHandler() {
        return new Handler(Looper.getMainLooper());
    }

    public boolean post(Runnable action) {
        return winrtMainHandler().post(action);
    }

    public boolean postDelayed(Runnable action, long delayMillis) {
        return winrtMainHandler().postDelayed(action, delayMillis);
    }

    public boolean removeCallbacks(Runnable action) {
        winrtMainHandler().removeCallbacks(action);
        return true;
    }

    public void invalidate() {
        winrtCheckThread();
        Native.invalidate();
    }

    public void requestLayout() {
        winrtCheckThread();
        Native.invalidate();
    }

    public final void measure(int widthSpec, int heightSpec) {
        onMeasure(widthSpec, heightSpec);
    }

    protected void onMeasure(int widthSpec, int heightSpec) {
        int w = mPadLeft + mPadRight;
        int h = mPadTop + mPadBottom;
        if (w < mMinWidth) {
            w = mMinWidth;
        }
        if (h < mMinHeight) {
            h = mMinHeight;
        }
        setMeasuredDimension(resolveSize(w, widthSpec), resolveSize(h, heightSpec));
    }

    protected final void setMeasuredDimension(int w, int h) {
        mMeasuredWidth = w;
        mMeasuredHeight = h;
    }

    public static int resolveSize(int desired, int spec) {
        int mode = MeasureSpec.getMode(spec);
        int size = MeasureSpec.getSize(spec);
        if (mode == MeasureSpec.EXACTLY) {
            return size;
        }
        if (mode == MeasureSpec.AT_MOST) {
            return desired < size ? desired : size;
        }
        return desired;
    }

    public void layout(int l, int t, int r, int b) {
        mLeft = l;
        mTop = t;
        mRight = r;
        mBottom = b;
        onLayout(true, l, t, r, b);
    }

    protected void onLayout(boolean changed, int l, int t, int r, int b) {}

    void updateAbs(int ox, int oy) {
        mAbsX = ox + mLeft;
        mAbsY = oy + mTop;
    }

    final void drawTree() {
        if (mVisibility != VISIBLE) {
            return;
        }
        if (mHasBackground) {
            Native.fillRect(mAbsX, mAbsY, getWidth(), getHeight(), mBackground);
        }
        onDrawNative(mAbsX, mAbsY);
        dispatchDraw();
    }

    protected void onDrawNative(int ax, int ay) {}

    protected void dispatchDraw() {}

    View findTouchTarget(int x, int y) {
        if (mVisibility != VISIBLE || !mEnabled || !mClickable) {
            return null;
        }
        return containsAbs(x, y) ? this : null;
    }

    final boolean containsAbs(int x, int y) {
        return x >= mAbsX && y >= mAbsY && x < mAbsX + getWidth() && y < mAbsY + getHeight();
    }

    public static void winrtUpdateAbs(View v) {
        v.updateAbs(0, 0);
    }

    public static void winrtDrawTree(View v) {
        v.drawTree();
    }

    public static View winrtFindTarget(View v, int x, int y) {
        return v.findTouchTarget(x, y);
    }

    public static void winrtSetPressed(View v, boolean p) {
        v.mPressed = p;
    }

    public static void winrtDump(View v, int depth) {
        for (int i = 0; i < depth; i++) {
            Native.dumpStr("  ");
        }
        String kind = v instanceof android.widget.Button ? "Button" : v instanceof android.widget.TextView ? "TextView" : v instanceof android.widget.LinearLayout ? "LinearLayout" : v instanceof ViewGroup ? "ViewGroup" : "View";
        Native.dumpStr(kind);
        if (v.mId != NO_ID) {
            Native.dumpStr(" id=0x");
            Native.dumpHex(v.mId);
        }
        Native.dumpStr(" [");
        Native.dumpInt(v.mAbsX);
        Native.dumpStr(",");
        Native.dumpInt(v.mAbsY);
        Native.dumpStr(" ");
        Native.dumpInt(v.getWidth());
        Native.dumpStr("x");
        Native.dumpInt(v.getHeight());
        Native.dumpStr("]");
        if (v.mVisibility != VISIBLE) {
            Native.dumpStr(" vis=");
            Native.dumpInt(v.mVisibility);
        }
        if (v instanceof android.widget.TextView) {
            Native.dumpStr(" text=\"");
            Native.dumpStr(((android.widget.TextView) v).getText().toString());
            Native.dumpStr("\"");
        }
        Native.dumpStr("\n");
        if (v instanceof ViewGroup) {
            ViewGroup g = (ViewGroup) v;
            for (int i = 0; i < g.getChildCount(); i++) {
                winrtDump(g.getChildAt(i), depth + 1);
            }
        }
    }

    public static boolean winrtContains(View v, int x, int y) {

        return v.containsAbs(x, y);
    }
}
