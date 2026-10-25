package android.widget;

import android.content.Context;
import android.util.AttributeSet;
import android.view.View;
import android.winrt.Host;
import android.winrt.Native;
import android.winrt.XmlAttrs;

public class TextView extends View {
    protected CharSequence mText = "";
    protected float mSize = 14f;
    protected boolean mSizeIsPx;
    protected int mColor = 0xFF000000;
    protected int mGravity;

    public TextView(Context context) {
        this(context, null);
    }

    public TextView(Context context, AttributeSet attrs) {
        super(context, attrs);
        mGravity = defaultGravity();
        if (attrs instanceof XmlAttrs) {
            applyTextAttrs((XmlAttrs) attrs);
        }
    }

    private void applyTextAttrs(XmlAttrs a) {
        int n = a.getAttributeCount();
        for (int i = 0; i < n; i++) {
            if (!a.isAndroid(i)) {
                continue;
            }
            String name = a.getAttributeName(i);
            if (name.equals("text")) {
                mText = a.text(i);
            } else if (name.equals("textSize")) {
                mSize = a.dimenPx(i, 14);
                mSizeIsPx = true;
            } else if (name.equals("textColor")) {
                mColor = a.color(i, mColor);
            } else if (name.equals("gravity")) {
                mGravity = a.intVal(i, mGravity);
            }
        }
    }

    protected int defaultGravity() {
        return 0;
    }

    protected int sizePx() {
        return mSizeIsPx ? (int) mSize : (int) (mSize * Host.DENSITY);
    }

    public void setText(CharSequence t) {
        mText = t == null ? "" : t;
        requestLayout();
    }

    public void setText(int resid) {
        setText(mContext.getResources().getText(resid));
    }

    public CharSequence getText() {
        return mText;
    }

    public void setTextSize(float sp) {
        mSize = sp;
        mSizeIsPx = false;
        requestLayout();
    }

    public void setTextSize(int unit, float v) {
        mSize = v;
        mSizeIsPx = unit == 0;
        requestLayout();
    }

    public void setTextColor(int c) {
        mColor = c;
        invalidate();
    }

    public void setGravity(int g) {
        mGravity = g;
        invalidate();
    }

    public int getGravity() {
        return mGravity;
    }

    protected int minWidth() {
        return mMinWidth;
    }

    protected int minHeight() {
        return mMinHeight;
    }

    @Override
    protected void onMeasure(int ws, int hs) {
        String s = mText.toString();
        int sz = sizePx();
        int w = Native.measureText(s, sz) + mPadLeft + mPadRight;
        int h = sz + sz / 4 + mPadTop + mPadBottom;
        if (w < minWidth()) {
            w = minWidth();
        }
        if (h < minHeight()) {
            h = minHeight();
        }
        setMeasuredDimension(resolveSize(w, ws), resolveSize(h, hs));
    }

    @Override
    protected void onDrawNative(int ax, int ay) {
        String s = mText.toString();
        int sz = sizePx();
        int tw = Native.measureText(s, sz);
        int th = sz + sz / 4;
        int availW = getWidth() - mPadLeft - mPadRight;
        int availH = getHeight() - mPadTop - mPadBottom;
        int x = ax + mPadLeft;
        int y = ay + mPadTop;
        int h = mGravity & 7;
        int v = mGravity & 0x70;
        if (h == 1) {
            x += (availW - tw) / 2;
        } else if (h == 5) {
            x += availW - tw;
        }
        if (v == 0x10) {
            y += (availH - th) / 2;
        } else if (v == 0x50) {
            y += availH - th;
        }
        Native.drawText(s, x, y, sz, mColor);
    }
}
