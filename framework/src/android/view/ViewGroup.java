package android.view;

import android.content.Context;
import android.util.AttributeSet;
import android.winrt.XmlAttrs;

public abstract class ViewGroup extends View {
    public static class LayoutParams {
        public static final int MATCH_PARENT = -1;
        public static final int WRAP_CONTENT = -2;
        public int width;
        public int height;

        public LayoutParams(int w, int h) {
            width = w;
            height = h;
        }

        public LayoutParams(Context c, AttributeSet attrs) {
            width = WRAP_CONTENT;
            height = WRAP_CONTENT;
            if (attrs instanceof XmlAttrs) {
                XmlAttrs a = (XmlAttrs) attrs;
                int i = a.indexOf(XmlAttrs.NS, "layout_width");
                if (i >= 0) {
                    width = a.dimenPx(i, WRAP_CONTENT);
                }
                i = a.indexOf(XmlAttrs.NS, "layout_height");
                if (i >= 0) {
                    height = a.dimenPx(i, WRAP_CONTENT);
                }
            }
        }
    }

    public static class MarginLayoutParams extends LayoutParams {
        public int leftMargin;
        public int topMargin;
        public int rightMargin;
        public int bottomMargin;

        public MarginLayoutParams(int w, int h) {
            super(w, h);
        }

        public MarginLayoutParams(Context c, AttributeSet attrs) {
            super(c, attrs);
            if (attrs instanceof XmlAttrs) {
                XmlAttrs a = (XmlAttrs) attrs;
                int all = a.indexOf(XmlAttrs.NS, "layout_margin");
                int m = all >= 0 ? a.dimenPx(all, 0) : 0;
                leftMargin = m;
                topMargin = m;
                rightMargin = m;
                bottomMargin = m;
                int i = a.indexOf(XmlAttrs.NS, "layout_marginLeft");
                if (i < 0) {
                    i = a.indexOf(XmlAttrs.NS, "layout_marginStart");
                }
                if (i >= 0) {
                    leftMargin = a.dimenPx(i, 0);
                }
                i = a.indexOf(XmlAttrs.NS, "layout_marginTop");
                if (i >= 0) {
                    topMargin = a.dimenPx(i, 0);
                }
                i = a.indexOf(XmlAttrs.NS, "layout_marginRight");
                if (i < 0) {
                    i = a.indexOf(XmlAttrs.NS, "layout_marginEnd");
                }
                if (i >= 0) {
                    rightMargin = a.dimenPx(i, 0);
                }
                i = a.indexOf(XmlAttrs.NS, "layout_marginBottom");
                if (i >= 0) {
                    bottomMargin = a.dimenPx(i, 0);
                }
            }
        }

        public void setMargins(int l, int t, int r, int b) {
            leftMargin = l;
            topMargin = t;
            rightMargin = r;
            bottomMargin = b;
        }
    }

    private View[] mKids = new View[4];
    private int mCount;

    public ViewGroup(Context context) {
        this(context, null);
    }

    public ViewGroup(Context context, AttributeSet attrs) {
        super(context, attrs);
    }

    public LayoutParams generateLayoutParams(AttributeSet attrs) {
        return new MarginLayoutParams(mContext, attrs);
    }

    protected LayoutParams generateDefaultLayoutParams() {
        return new LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT);
    }

    protected LayoutParams adapt(LayoutParams lp) {
        return lp;
    }

    public void addView(View v) {
        addView(v, generateDefaultLayoutParams());
    }

    public void addView(View v, LayoutParams lp) {
        v.mLayoutParams = adapt(lp);
        v.mParent = this;
        if (mCount == mKids.length) {
            View[] n = new View[mCount * 2];
            for (int i = 0; i < mCount; i++) {
                n[i] = mKids[i];
            }
            mKids = n;
        }
        mKids[mCount++] = v;
        requestLayout();
    }

    public void removeAllViews() {
        for (int i = 0; i < mCount; i++) {
            mKids[i].mParent = null;
            mKids[i] = null;
        }
        mCount = 0;
        requestLayout();
    }

    public int getChildCount() {
        return mCount;
    }

    public View getChildAt(int i) {
        return i < 0 || i >= mCount ? null : mKids[i];
    }

    @Override
    public View findViewById(int id) {
        if (mId == id) {
            return this;
        }
        for (int i = 0; i < mCount; i++) {
            View v = mKids[i].findViewById(id);
            if (v != null) {
                return v;
            }
        }
        return null;
    }

    public static int getChildMeasureSpec(int spec, int padding, int childDim) {
        int mode = MeasureSpec.getMode(spec);
        int size = MeasureSpec.getSize(spec) - padding;
        if (size < 0) {
            size = 0;
        }
        if (mode == MeasureSpec.EXACTLY) {
            if (childDim >= 0) {
                return MeasureSpec.makeMeasureSpec(childDim, MeasureSpec.EXACTLY);
            }
            if (childDim == LayoutParams.MATCH_PARENT) {
                return MeasureSpec.makeMeasureSpec(size, MeasureSpec.EXACTLY);
            }
            return MeasureSpec.makeMeasureSpec(size, MeasureSpec.AT_MOST);
        }
        if (mode == MeasureSpec.AT_MOST) {
            if (childDim >= 0) {
                return MeasureSpec.makeMeasureSpec(childDim, MeasureSpec.EXACTLY);
            }
            return MeasureSpec.makeMeasureSpec(size, MeasureSpec.AT_MOST);
        }
        if (childDim >= 0) {
            return MeasureSpec.makeMeasureSpec(childDim, MeasureSpec.EXACTLY);
        }
        return MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED);
    }

    @Override
    void updateAbs(int ox, int oy) {
        super.updateAbs(ox, oy);
        for (int i = 0; i < mCount; i++) {
            mKids[i].updateAbs(mAbsX, mAbsY);
        }
    }

    @Override
    protected void dispatchDraw() {
        for (int i = 0; i < mCount; i++) {
            mKids[i].drawTree();
        }
    }

    @Override
    View findTouchTarget(int x, int y) {
        if (mVisibility != VISIBLE || !mEnabled) {
            return null;
        }
        for (int i = mCount - 1; i >= 0; i--) {
            View t = mKids[i].findTouchTarget(x, y);
            if (t != null) {
                return t;
            }
        }
        return mClickable && containsAbs(x, y) ? this : null;
    }
}
