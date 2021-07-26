<template>
  <select
    class="form-select"
    aria-label="Chart Type"
    v-model="activeChartType"
    @change="onChartTypeChange"
  >
    <option disabled value="">Chart Type</option>
    <option v-for="chartType in chartTypes" :key="chartType">
      {{ chartType }}
    </option>
  </select>
  <div id="chart"></div>
</template>


<script>
import { bb, donut, pie, line } from "billboard.js";

const chartTypes = {
  PIE: "pie",
  DONUT: "donut",
  LINE: "line",
};

export default {
  name: "NotebookCellChart",
  props: ["rows"],
  created() {
    this.chartTypes = chartTypes;
  },
  data() {
    return {
      activeChartType: "",
    };
  },
  methods: {
    onChartTypeChange() {
      var type = null;
      if (this.activeChartType == chartTypes.PIE) {
        type = pie();
      } else if (this.activeChartType == chartTypes.DONUT) {
        type = donut();
      } else if (this.activeChartType == chartTypes.LINE) {
        type = line();
      }

      bb.generate({
        bindto: "#chart",
        data: {
          type: type,
          json: this.rows,
          keys: { value: Object.keys(this.rows[0]) },
        },
      });
    },
  },
};
</script>

<style scoped>
</style>