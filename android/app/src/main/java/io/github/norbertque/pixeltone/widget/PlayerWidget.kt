package io.github.norbertque.pixeltone.widget

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.view.KeyEvent
import android.widget.RemoteViews
import androidx.core.content.edit
import io.github.norbertque.pixeltone.MainActivity
import io.github.norbertque.pixeltone.R
import io.github.norbertque.pixeltone.playback.PlaybackService

/** What the widget shows; kept so a widget placed while nothing plays shows the last track. */
data class WidgetState(val title: String?, val artist: String?, val playing: Boolean)

/**
 * The home-screen widget: the cover, title and artist, and previous / play-pause / next. Its
 * buttons go to the playback service as media buttons (like the notification's), so they work
 * with the app closed; the service sends it what plays (also on a PC).
 */
class PlayerWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) {
        manager.updateAppWidget(ids, views(context, load(context), cover = null))
    }

    companion object {
        /** Shows [state] (and [cover], once loaded) on every Pixeltone widget there is. */
        fun update(context: Context, state: WidgetState, cover: Bitmap?) {
            save(context, state)
            val manager = AppWidgetManager.getInstance(context) ?: return
            val ids = manager.getAppWidgetIds(ComponentName(context, PlayerWidget::class.java))
            if (ids.isNotEmpty()) manager.updateAppWidget(ids, views(context, state, cover))
        }

        internal fun views(context: Context, state: WidgetState, cover: Bitmap?) =
            RemoteViews(context.packageName, R.layout.widget_player).apply {
                setTextViewText(R.id.widget_title, state.title ?: context.getString(R.string.app_name))
                setTextViewText(R.id.widget_artist, state.artist ?: context.getString(R.string.widget_idle))
                if (cover != null) {
                    setImageViewBitmap(R.id.widget_cover, cover)
                } else {
                    setImageViewResource(R.id.widget_cover, R.drawable.widget_cover_placeholder)
                }
                setImageViewResource(R.id.widget_play, if (state.playing) R.drawable.ic_widget_pause else R.drawable.ic_widget_play)
                setContentDescription(R.id.widget_play, context.getString(if (state.playing) R.string.pause else R.string.play))
                setOnClickPendingIntent(R.id.widget_open, openApp(context))
                if (state.title == null) {
                    // Nothing has played yet: the play button opens the app instead.
                    setOnClickPendingIntent(R.id.widget_play, openApp(context))
                } else {
                    setOnClickPendingIntent(R.id.widget_play, button(context, KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE))
                }
                setOnClickPendingIntent(R.id.widget_previous, button(context, KeyEvent.KEYCODE_MEDIA_PREVIOUS))
                setOnClickPendingIntent(R.id.widget_next, button(context, KeyEvent.KEYCODE_MEDIA_NEXT))
            }

        private fun openApp(context: Context): PendingIntent =
            PendingIntent.getActivity(
                context,
                0,
                Intent(context, MainActivity::class.java),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )

        /** A media button for the session, the way Media3's notification sends them. */
        private fun button(context: Context, keyCode: Int): PendingIntent {
            val intent = Intent(context, PlaybackService::class.java)
                .setAction(Intent.ACTION_MEDIA_BUTTON)
                .putExtra(Intent.EXTRA_KEY_EVENT, KeyEvent(KeyEvent.ACTION_DOWN, keyCode))
            return PendingIntent.getForegroundService(
                context,
                keyCode,
                intent,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        }

        private fun save(context: Context, state: WidgetState) {
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit {
                putString(TITLE, state.title)
                putString(ARTIST, state.artist)
            }
        }

        // Shown paused: the service tells it otherwise as soon as something plays.
        private fun load(context: Context): WidgetState {
            val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            return WidgetState(prefs.getString(TITLE, null), prefs.getString(ARTIST, null), playing = false)
        }

        private const val PREFS = "widget"
        private const val TITLE = "title"
        private const val ARTIST = "artist"
    }
}
