package android.view;

import android.content.Context;
import android.winrt.Native;

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

    public View(Context context) {
        mContext = context;
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

    public void invalidate() {
        Native.invalidate();
    }

    public void requestLayout() {
        Native.invalidate();
    }

    public final void measure(int widthSpec, int heightSpec) {
        onMeasure(widthSpec, heightSpec);
    }

    protected void onMeasure(int widthSpec, int heightSpec) {
        setMeasuredDimension(resolveSize(mPadLeft + mPadRight, widthSpec), resolveSize(mPadTop + mPadBottom, heightSpec));
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

    public static boolean winrtContains(View v, int x, int y) {
        return v.containsAbs(x, y);
    }
}
