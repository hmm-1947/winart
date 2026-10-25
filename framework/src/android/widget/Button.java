package android.widget;

import android.content.Context;
import android.util.AttributeSet;
import android.winrt.Host;
import android.winrt.Native;

public class Button extends TextView {
    public Button(Context context) {
        this(context, null);
    }

    public Button(Context context, AttributeSet attrs) {
        super(context, attrs);
        mClickable = true;
    }

    @Override
    protected int defaultGravity() {
        return 0x11;
    }

    @Override
    protected void onMeasure(int ws, int hs) {
        if (mPadLeft == 0 && mPadTop == 0 && mPadRight == 0 && mPadBottom == 0) {
            int p = (int) (8f * Host.DENSITY);
            mPadLeft = p;
            mPadTop = p;
            mPadRight = p;
            mPadBottom = p;
        }
        super.onMeasure(ws, hs);
    }

    @Override
    protected int minWidth() {
        return (int) (64f * Host.DENSITY);
    }

    @Override
    protected int minHeight() {
        return (int) (48f * Host.DENSITY);
    }

    @Override
    protected void onDrawNative(int ax, int ay) {
        int w = getWidth();
        int h = getHeight();
        int inset = (int) (Host.DENSITY * 2f);
        Native.fillRect(ax + inset, ay + inset, w - 2 * inset, h - 2 * inset, mPressed ? 0xFFBDBDBD : mEnabled ? 0xFFE0E0E0 : 0xFFF0F0F0);
        Native.strokeRect(ax + inset, ay + inset, w - 2 * inset, h - 2 * inset, 0xFF9E9E9E, 2);
        super.onDrawNative(ax, ay);
    }
}
