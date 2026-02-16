export function requestNotificationPermission(): void {
  if ("Notification" in window && Notification.permission === "default") {
    Notification.requestPermission();
  }
}

export function sendBrowserNotification(title: string, body: string): void {
  if (
    "Notification" in window &&
    Notification.permission === "granted" &&
    document.hidden
  ) {
    new Notification(title, { body, icon: "/favicon.ico" });
  }
}

let audioContext: AudioContext | null = null;

export function playNotificationSound(): void {
  try {
    if (!audioContext) {
      audioContext = new AudioContext();
    }
    const oscillator = audioContext.createOscillator();
    const gain = audioContext.createGain();
    oscillator.connect(gain);
    gain.connect(audioContext.destination);

    oscillator.frequency.setValueAtTime(880, audioContext.currentTime);
    oscillator.type = "sine";
    gain.gain.setValueAtTime(0.1, audioContext.currentTime);
    gain.gain.exponentialRampToValueAtTime(
      0.001,
      audioContext.currentTime + 0.3
    );

    oscillator.start(audioContext.currentTime);
    oscillator.stop(audioContext.currentTime + 0.3);
  } catch {
    /* silent */
  }
}
