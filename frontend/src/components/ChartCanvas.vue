<script setup lang="ts">
import {onBeforeUnmount, onMounted, ref, watch} from 'vue';
import {Chart, registerables, type ChartConfiguration} from 'chart.js';

Chart.register(...registerables);

const props = defineProps<{
  config: ChartConfiguration;
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let chart: Chart | null = null;

function renderChart() {
  if (!canvasRef.value) return;

  chart?.destroy();
  chart = new Chart(canvasRef.value, props.config);
}

onMounted(renderChart);
watch(() => props.config, renderChart, {deep: true});
onBeforeUnmount(() => chart?.destroy());
</script>

<template>
  <div class="chart-wrap">
    <canvas ref="canvasRef"></canvas>
  </div>
</template>

<style scoped>
.chart-wrap {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 0;
}

canvas {
  display: block;
  width: 100% !important;
  height: 100% !important;
  max-height: 100%;
}
</style>

