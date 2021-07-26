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
    <h2>Notebooks</h2>
  </div>

  <ul>
    <li v-for="notebook in notebooks" :key="notebook">
      {{ notebook }}
    </li>
  </ul>

  <form v-on:submit.prevent="addNotebook">
    <button class="add-cell btn btn-sm btn-secondary">
      <i class="bi-plus-circle-fill"></i> Add Notebook
    </button>
  </form>
</template>

<script>
import { uuid } from "vue-uuid";

export default {
  name: "Notebooks",
  data() {
    return {
      notebooks: [],
    };
  },
  methods: {
    addNotebook() {
      var notebook = {
        id: uuid.v4(),
      };
      this.notebooks.push(notebook);
      this.$router.push("/notebook/" + notebook.id);
      console.log(this.notebooks);
    },
  },
};
</script>
