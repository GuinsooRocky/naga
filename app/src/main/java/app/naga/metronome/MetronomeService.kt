package app.naga.metronome

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import app.naga.MainActivity
import app.naga.R

/** 让节拍器在锁屏和切后台后继续响，并在通知栏提供暂停。 */
class MetronomeService : Service() {
    private var wakeLock: PowerManager.WakeLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf()
            return START_NOT_STICKY
        }
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(1, notification)
        }
        if (wakeLock == null) {
            // 不持锁的话锁屏后 CPU 会休眠，节拍会断
            wakeLock = getSystemService(PowerManager::class.java)
                .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "naga:metronome")
                .also { it.acquire(4 * 60 * 60 * 1000L) }
        }
        MetronomeEngine.start()
        return START_NOT_STICKY
    }

    override fun onTaskRemoved(rootIntent: Intent?) {
        stopSelf()
    }

    override fun onDestroy() {
        MetronomeEngine.stop()
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
        super.onDestroy()
    }

    private fun buildNotification(): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "节拍器", NotificationManager.IMPORTANCE_LOW)
        )
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE
        )
        val stop = PendingIntent.getService(
            this, 1,
            Intent(this, MetronomeService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE,
        )
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_metronome)
            .setContentTitle("节拍器正在响")
            .setContentText("${MetronomeEngine.bpm.value} BPM · ${MetronomeEngine.signature.value.label}")
            .setContentIntent(open)
            .setOngoing(true)
            .addAction(
                Notification.Action.Builder(
                    Icon.createWithResource(this, R.drawable.ic_metronome), "暂停", stop
                ).build()
            )
            .build()
    }

    companion object {
        private const val CHANNEL = "metronome"
        private const val ACTION_STOP = "app.naga.STOP"

        fun start(context: Context) {
            context.startForegroundService(Intent(context, MetronomeService::class.java))
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, MetronomeService::class.java))
        }
    }
}
