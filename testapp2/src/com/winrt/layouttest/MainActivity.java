package com.winrt.layouttest;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import android.widget.TextView;

public class MainActivity extends Activity {
    private TextView counter;
    private int taps;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        counter = (TextView) findViewById(R.id.counter);
        Button tap = (Button) findViewById(R.id.btn_tap);
        tap.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                taps++;
                counter.setText(getString(R.string.count_fmt, taps));
            }
        });
    }

    public void onReset(View v) {
        taps = 0;
        counter.setText(R.string.title);
    }
}
