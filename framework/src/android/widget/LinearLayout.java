package android.widget;

import android.content.Context;
import android.view.View;
import android.view.ViewGroup;

public class LinearLayout extends ViewGroup {
    public static final int HORIZONTAL = 0;
    public static final int VERTICAL = 1;

    public static class LayoutParams extends ViewGroup.MarginLayoutParams {
        public float weight;

        public LayoutParams(int w, int h) {
            super(w, h);
        }

        public LayoutParams(int w, int h, float weight) {
            super(w, h);
            this.weight = weight;
        }
    }

    private int mOrientation = HORIZONTAL;

    public LinearLayout(Context context) {
        super(context);
    }

    public void setOrientation(int o) {
        mOrientation = o;
        requestLayout();
    }

    public int getOrientation() {
        return mOrientation;
    }

    @Override
    protected ViewGroup.LayoutParams generateDefaultLayoutParams() {
        if (mOrientation == VERTICAL) {
            return new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        }
        return new LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    @Override
    protected ViewGroup.LayoutParams adapt(ViewGroup.LayoutParams lp) {
        if (lp instanceof LayoutParams) {
            return lp;
        }
        LayoutParams n = new LayoutParams(lp.width, lp.height);
        if (lp instanceof ViewGroup.MarginLayoutParams) {
            ViewGroup.MarginLayoutParams m = (ViewGroup.MarginLayoutParams) lp;
            n.setMargins(m.leftMargin, m.topMargin, m.rightMargin, m.bottomMargin);
        }
        return n;
    }

    private static LayoutParams lp(View v) {
        return (LayoutParams) v.getLayoutParams();
    }

    @Override
    protected void onMeasure(int ws, int hs) {
        boolean vert = mOrientation == VERTICAL;
        int n = getChildCount();
        int mainSpec = vert ? hs : ws;
        int crossSpec = vert ? ws : hs;
        int padMain = vert ? getPaddingTop() + getPaddingBottom() : getPaddingLeft() + getPaddingRight();
        int padCross = vert ? getPaddingLeft() + getPaddingRight() : getPaddingTop() + getPaddingBottom();
        float weightSum = 0f;
        int total = 0;
        int maxCross = 0;
        for (int i = 0; i < n; i++) {
            View c = getChildAt(i);
            if (c.getVisibility() == GONE) {
                continue;
            }
            LayoutParams p = lp(c);
            weightSum += p.weight;
            int mMain = vert ? p.topMargin + p.bottomMargin : p.leftMargin + p.rightMargin;
            int mCross = vert ? p.leftMargin + p.rightMargin : p.topMargin + p.bottomMargin;
            int cw = getChildMeasureSpec(ws, (vert ? padCross + mCross : padMain + mMain), p.width);
            int ch = getChildMeasureSpec(hs, (vert ? padMain + mMain : padCross + mCross), p.height);
            int mainDim = vert ? p.height : p.width;
            if (p.weight > 0f && mainDim == 0) {
                if (vert) {
                    ch = MeasureSpec.makeMeasureSpec(0, MeasureSpec.EXACTLY);
                } else {
                    cw = MeasureSpec.makeMeasureSpec(0, MeasureSpec.EXACTLY);
                }
            }
            c.measure(cw, ch);
            int cm = vert ? c.getMeasuredHeight() : c.getMeasuredWidth();
            int cc = vert ? c.getMeasuredWidth() : c.getMeasuredHeight();
            total += cm + mMain;
            if (cc + mCross > maxCross) {
                maxCross = cc + mCross;
            }
        }
        int mainSize = resolveSize(total + padMain, mainSpec);
        if (weightSum > 0f) {
            int remaining = mainSize - padMain - total;
            for (int i = 0; i < n; i++) {
                View c = getChildAt(i);
                if (c.getVisibility() == GONE) {
                    continue;
                }
                LayoutParams p = lp(c);
                if (p.weight <= 0f) {
                    continue;
                }
                int mCross = vert ? p.leftMargin + p.rightMargin : p.topMargin + p.bottomMargin;
                int base = vert ? c.getMeasuredHeight() : c.getMeasuredWidth();
                int extra = (int) (remaining * p.weight / weightSum);
                int ms = MeasureSpec.makeMeasureSpec(base + extra < 0 ? 0 : base + extra, MeasureSpec.EXACTLY);
                int cs = getChildMeasureSpec(crossSpec, padCross + mCross, vert ? p.width : p.height);
                if (vert) {
                    c.measure(cs, ms);
                } else {
                    c.measure(ms, cs);
                }
            }
        }
        int crossSize = resolveSize(maxCross + padCross, crossSpec);
        if (vert) {
            setMeasuredDimension(crossSize, mainSize);
        } else {
            setMeasuredDimension(mainSize, crossSize);
        }
    }

    @Override
    protected void onLayout(boolean changed, int l, int t, int r, int b) {
        boolean vert = mOrientation == VERTICAL;
        int n = getChildCount();
        int pos = vert ? getPaddingTop() : getPaddingLeft();
        for (int i = 0; i < n; i++) {
            View c = getChildAt(i);
            if (c.getVisibility() == GONE) {
                continue;
            }
            LayoutParams p = lp(c);
            int w = c.getMeasuredWidth();
            int h = c.getMeasuredHeight();
            if (vert) {
                pos += p.topMargin;
                int x = getPaddingLeft() + p.leftMargin;
                c.layout(x, pos, x + w, pos + h);
                pos += h + p.bottomMargin;
            } else {
                pos += p.leftMargin;
                int y = getPaddingTop() + p.topMargin;
                c.layout(pos, y, pos + w, y + h);
                pos += w + p.rightMargin;
            }
        }
    }
}
