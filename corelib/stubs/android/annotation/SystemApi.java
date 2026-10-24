package android.annotation;
import java.lang.annotation.*;
@Retention(RetentionPolicy.SOURCE)
public @interface SystemApi {
    enum Client { PRIVILEGED_APPS, MODULE_LIBRARIES }
    Client client() default Client.PRIVILEGED_APPS;
    String process() default "";
}
