interface ClockProps {
  now: Date;
  showSeconds: boolean;
  showDate: boolean;
}

const WEEKDAYS = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

const pad = (value: number) => String(value).padStart(2, "0");

export default function Clock({ now, showSeconds, showDate }: ClockProps) {
  const hoursMinutes = `${pad(now.getHours())}:${pad(now.getMinutes())}`;
  const seconds = pad(now.getSeconds());
  const date = `${now.getMonth() + 1}月${now.getDate()}日 ${WEEKDAYS[now.getDay()]}`;

  return (
    <div className="clock">
      <div className="time">
        <span className="hm">{hoursMinutes}</span>
        {showSeconds && (
          <span className="ss">
            <span className="colon">:</span>
            {seconds}
          </span>
        )}
      </div>
      {showDate && <div className="date">{date}</div>}
    </div>
  );
}
