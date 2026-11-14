package dev.astrid.overlay

import android.content.Context
import android.content.Intent
import android.os.Bundle
import com.facebook.react.ReactActivity
import com.facebook.react.ReactActivityDelegate
import com.facebook.react.defaults.DefaultNewArchitectureEntryPoint.fabricEnabled
import com.facebook.react.defaults.DefaultReactActivityDelegate

/**
 * The ask panel over other apps (ADR 0039): a see-through activity that shows
 * the app's `AstridAsk` component on the same React runtime as the app, so it
 * shares the phone link. What is beneath it stays visible; tapping outside the
 * panel closes it.
 */
class AskActivity : ReactActivity() {
  override fun getMainComponentName(): String = "AstridAsk"

  // Wrapped as the app's MainActivity is, so Expo modules get this
  // activity's lifecycle too. The wrapper lives in the `expo` package, which
  // depends on this module, so it is found at run time.
  override fun createReactActivityDelegate(): ReactActivityDelegate {
    val inner = object : DefaultReactActivityDelegate(this, mainComponentName, fabricEnabled) {}
    return try {
      Class.forName("expo.modules.ReactActivityDelegateWrapper")
        .getConstructor(ReactActivity::class.java, Boolean::class.javaPrimitiveType, ReactActivityDelegate::class.java)
        .newInstance(this, true, inner) as ReactActivityDelegate
    } catch (error: Throwable) {
      android.util.Log.w("AstridOverlay", "expo delegate wrapper unavailable: $error")
      inner
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(null)
    overridePendingTransition(0, 0)
  }

  override fun finish() {
    super.finish()
    overridePendingTransition(0, 0)
  }

  companion object {
    fun intent(context: Context): Intent =
      Intent(context, AskActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
  }
}
