import { useEffect, useRef } from "react";
import {
  CandlestickSeries,
  ColorType,
  LineSeries,
  createChart,
  type CandlestickData,
  type IChartApi,
  type ISeriesApi,
  type LineData,
  type UTCTimestamp
} from "lightweight-charts";

import { useAppStore } from "../store/appStore";
import { Panel } from "./Panel";

export function ChartSection() {
  const candles = useAppStore((state) => state.candles);
  const asoPoints = useAppStore((state) => state.asoPoints);
  const chartVersion = useAppStore((state) => state.chartVersion);
  const lastCandleUpdate = useAppStore((state) => state.lastCandleUpdate);
  const lastAsoUpdate = useAppStore((state) => state.lastAsoUpdate);
  const activeSymbol = useAppStore((state) => state.activeSymbol);
  const timeframe = useAppStore((state) => state.runtimeStatus?.timeframe);
  const priceHostRef = useRef<HTMLDivElement | null>(null);
  const asoHostRef = useRef<HTMLDivElement | null>(null);
  const priceChartRef = useRef<IChartApi | null>(null);
  const asoChartRef = useRef<IChartApi | null>(null);
  const candleSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const bullSeriesRef = useRef<ISeriesApi<"Line"> | null>(null);
  const bearSeriesRef = useRef<ISeriesApi<"Line"> | null>(null);

  useEffect(() => {
    if (!priceHostRef.current || !asoHostRef.current) {
      return;
    }

    const common = {
      layout: {
        background: { type: ColorType.Solid, color: "#09131e" },
        textColor: "#dce9f5"
      },
      grid: {
        vertLines: { color: "rgba(122, 201, 255, 0.08)" },
        horzLines: { color: "rgba(122, 201, 255, 0.08)" }
      },
      timeScale: {
        borderColor: "rgba(122, 201, 255, 0.15)"
      },
      rightPriceScale: {
        borderColor: "rgba(122, 201, 255, 0.15)"
      }
    } as const;

    const priceChart = createChart(priceHostRef.current, {
      ...common,
      height: 260
    });
    const asoChart = createChart(asoHostRef.current, {
      ...common,
      height: 150
    });
    const candleSeries = priceChart.addSeries(CandlestickSeries, {
      upColor: "#7de2c1",
      downColor: "#ff8c73",
      borderVisible: false,
      wickUpColor: "#7de2c1",
      wickDownColor: "#ff8c73"
    });
    const bullSeries = asoChart.addSeries(LineSeries, {
      color: "#7de2c1",
      lineWidth: 2,
      title: "Bulls"
    });
    const bearSeries = asoChart.addSeries(LineSeries, {
      color: "#f5c96a",
      lineWidth: 2,
      title: "Bears"
    });

    let syncing = false;
    const syncRange = (source: IChartApi, target: IChartApi) => {
      source.timeScale().subscribeVisibleLogicalRangeChange((range) => {
        if (!range || syncing) {
          return;
        }
        syncing = true;
        target.timeScale().setVisibleLogicalRange(range);
        syncing = false;
      });
    };
    syncRange(priceChart, asoChart);
    syncRange(asoChart, priceChart);

    priceChartRef.current = priceChart;
    asoChartRef.current = asoChart;
    candleSeriesRef.current = candleSeries;
    bullSeriesRef.current = bullSeries;
    bearSeriesRef.current = bearSeries;

    const resizeObserver = new ResizeObserver(() => {
      const width = priceHostRef.current?.clientWidth ?? 0;
      priceChart.applyOptions({ width });
      asoChart.applyOptions({ width: asoHostRef.current?.clientWidth ?? width });
    });
    resizeObserver.observe(priceHostRef.current);
    resizeObserver.observe(asoHostRef.current);

    return () => {
      resizeObserver.disconnect();
      priceChart.remove();
      asoChart.remove();
    };
  }, []);

  useEffect(() => {
    if (!candleSeriesRef.current || !bullSeriesRef.current || !bearSeriesRef.current) {
      return;
    }
    candleSeriesRef.current.setData(candles.map(toCandleData));
    bullSeriesRef.current.setData(asoPoints.filter((point) => point.ready && point.bulls !== null).map((point) => ({ time: toTime(point.open_time), value: point.bulls! } satisfies LineData)));
    bearSeriesRef.current.setData(asoPoints.filter((point) => point.ready && point.bears !== null).map((point) => ({ time: toTime(point.open_time), value: point.bears! } satisfies LineData)));
  }, [asoPoints, candles, chartVersion]);

  useEffect(() => {
    if (lastCandleUpdate && candleSeriesRef.current) {
      candleSeriesRef.current.update(toCandleData(lastCandleUpdate.candle));
    }
  }, [lastCandleUpdate]);

  useEffect(() => {
    if (lastAsoUpdate && bullSeriesRef.current && bearSeriesRef.current && lastAsoUpdate.point.ready) {
      if (lastAsoUpdate.point.bulls !== null) {
        bullSeriesRef.current.update({ time: toTime(lastAsoUpdate.point.open_time), value: lastAsoUpdate.point.bulls });
      }
      if (lastAsoUpdate.point.bears !== null) {
        bearSeriesRef.current.update({ time: toTime(lastAsoUpdate.point.open_time), value: lastAsoUpdate.point.bears });
      }
    }
  }, [lastAsoUpdate]);

  return (
    <div className="grid-span-8">
      <Panel title="Chart Section">
        <div className="chart-stack">
          <div className="muted">
            {activeSymbol ?? "n/a"} · {timeframe ?? "n/a"} · price and ASO share the same time axis
          </div>
          <div ref={priceHostRef} className="chart-slot" />
          <div ref={asoHostRef} className="chart-slot chart-slot--small" />
        </div>
      </Panel>
    </div>
  );
}

function toTime(timestamp: number): UTCTimestamp {
  return Math.floor(timestamp / 1000) as UTCTimestamp;
}

function toCandleData(candle: import("../types").Candle): CandlestickData<UTCTimestamp> {
  return {
    time: toTime(candle.open_time),
    open: candle.open,
    high: candle.high,
    low: candle.low,
    close: candle.close
  };
}
