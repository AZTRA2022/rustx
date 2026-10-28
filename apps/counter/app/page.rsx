fn Counter() -> Element {
   let mut n = 0;
   return <div onClick={|| n += 1}>{n}</div>;
}
