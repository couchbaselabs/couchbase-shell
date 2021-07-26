<template>
  <div class="btn-group btn-group-sm" role="group" aria-label="Cell Type">
    <button
      class="btn btn-outline-primary"
      :class="activeInputType == inputType ? 'active' : ''"
      v-for="inputType in inputTypes"
      @click="activeInputType = inputType"
      :key="inputType"
    >
      {{ inputType }}
    </button>
  </div>

  <div class="run btn-group btn-group-sm" role="group" aria-label="Cell Type">
    <form v-on:submit.prevent="exec">
      <button class="btn btn-success btn-sm">
        <i class="bi-arrow-clockwise"></i> Run
      </button>
    </form>
  </div>

  <textarea
    class="form-control"
    rows="1"
    v-model.lazy="input"
    @blur="exec"
    placeholder=".. enter your shell command here .."
  ></textarea>

  <div class="cell-box" v-if="results">
    <ul class="nav nav-tabs">
      <li
        class="nav-item"
        v-for="displayType in displayTypesFor(usedInputType)"
        @click="activeDisplayType = displayType"
        :key="displayType"
      >
        <a
          class="nav-link"
          :class="activeDisplayType == displayType ? 'active' : ''"
          aria-current="page"
          href="#"
          >{{ displayType }}</a
        >
      </li>
    </ul>

    <div class="" v-if="activeDisplayType == displayTypes.TABLE">
      <NotebookCellTable :rows="results" />
    </div>

    <div class="" v-if="activeDisplayType == displayTypes.JSON">
      <NotebookCellJson :rows="results" />
    </div>

    <div class="" v-if="activeDisplayType == displayTypes.CHART">
      <NotebookCellChart :rows="results" />
    </div>
  </div>
</template>

<script>
import NotebookCellTable from "./NotebookCellTable.vue";
import NotebookCellJson from "./NotebookCellJson.vue";
import NotebookCellChart from "./NotebookCellChart.vue";

const axios = require("axios");

const inputTypes = {
  SHELL: "shell",
  QUERY: "query",
  ANALYTICS: "analytics",
};

const displayTypes = {
  TABLE: "table",
  JSON: "json",
  CHART: "chart",
};

const displayTypesForInputType = {
  SHELL: [displayTypes.TABLE, displayTypes.JSON, displayTypes.CHART],
  QUERY: [displayTypes.TABLE, displayTypes.JSON, displayTypes.CHART],
  ANALYTICS: [displayTypes.TABLE, displayTypes.JSON, displayTypes.CHART],
};

export default {
  name: "NotebookCell",
  components: {
    NotebookCellTable,
    NotebookCellJson,
    NotebookCellChart,
  },
  created() {
    this.displayTypes = displayTypes;
    this.inputTypes = inputTypes;
    this.displayTypesForInputType = displayTypesForInputType;
  },
  data() {
    return {
      input: null,
      results: null,
      // This input type is stored and frozen as soon as the input
      // is sent so that even if the user changes the active one the
      // rendering of the results does not change until re-submitted
      usedInputType: null,
      // This is the input type which is active and the user can change it
      // all the time
      activeInputType: inputTypes.SHELL,
      // Holds the active tab on how the data is displayed (i.e. table, json etc)
      activeDisplayType: null,
    };
  },
  methods: {
    exec() {
      if (this.input == null || this.input.trim() === "") {
        return;
      }

      this.usedInputType = this.activeInputType;
      axios
        .post("http://localhost:3030/api/notebook/exec", {
          inputType: this.usedInputType,
          inputValue: this.input,
        })
        .then((res) => {
          // Todo: handle errors
          var converted = JSON.parse(res.data.result);
          if (!Array.isArray(converted)) {
            this.results = new Array(converted);
          } else {
            this.results = converted;
          }
          this.activeDisplayType = this.displayTypesFor(this.usedInputType)[0];
        })
        .catch(function (err) {
          console.log(err);
        });
    },
    displayTypesFor(it) {
      if (it == inputTypes.SHELL) {
        return displayTypesForInputType.SHELL;
      } else if (it == inputTypes.QUERY) {
        return displayTypesForInputType.QUERY;
      } else if (it == inputTypes.ANALYTICS) {
        return displayTypesForInputType.ANALYTICS;
      }
    },
  },
};
</script>


<style scoped>
div.cell-box {
  padding: 5px;
}

.run {
  margin-left: 10px;
}

h2,
.btn-group,
textarea {
  margin-bottom: 10px;
}
</style>