package android.compat.annotation;
import java.lang.annotation.*;
@Retention(RetentionPolicy.SOURCE)
public @interface UnsupportedAppUsage {
    int maxTargetSdk() default Integer.MAX_VALUE;
    long trackingBug() default 0;
    String expectedSignature() default "";
    String implicitMember() default "";
    Class<?> overriddenBy() default void.class;
    String publicAlternatives() default "";
}
