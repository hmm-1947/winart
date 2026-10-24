package android.compat.annotation;
import java.lang.annotation.*;
@Retention(RetentionPolicy.SOURCE)
public @interface EnabledSince { int targetSdkVersion(); }
