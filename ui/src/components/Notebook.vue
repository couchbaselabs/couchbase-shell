<template>
  <div
    class="
      d-flex
      justify-content-between
      flex-wrap flex-md-nowrap
      align-items-center
      pt-3
      pb-2
      mb-3
      border-bottom
    "
  >
    <h2>{{ name }}</h2>
    <div class="btn-toolbar mb-2 mb-md-0">
      <div class="btn-group me-2">
        <button type="button" class="btn btn-sm btn-outline-secondary disabled">
          Export
        </button>
      </div>
    </div>
  </div>

  <NotebookCell v-for="cell in cells" :key="cell" />

  <form v-on:submit.prevent="addNewCell">
    <button class="add-cell btn btn-sm btn-secondary">
      <i class="bi-plus-circle-fill"></i> Add Cell
    </button>
  </form>
</template>

<script>
import NotebookCell from "./NotebookCell.vue";

export default {
  name: "Notebook",
  components: {
    NotebookCell,
  },
  data() {
    return {
      id: this.$route.params.id,
      name: "My First Notebook",
      cells: [],
    };
  },
  methods: {
    addNewCell() {
      this.cells.push({});
    },
  },
  mounted() {
    this.addNewCell();
  },
};
</script>

<style scoped>
.add-cell {
  margin-top: 10px;
  margin-bottom: 20px;
}
</style>