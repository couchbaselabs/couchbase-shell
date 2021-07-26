<template>
  <table class="table table-sm table-hover">
    <thead>
      <tr>
        <th scope="col" v-for="column in columns" :key="column">
          {{ column }}
        </th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="row in rows" :key="row">
        <td v-for="column in columns" :key="column">
          {{ row[column] }}
        </td>
      </tr>
    </tbody>
  </table>
</template>


<script>
export default {
  name: "NotebookCellTable",
  props: ["rows"],
  computed: {
    columns: function () {
      return Object.keys(this.rows[0]);
    },
  },
};
</script>

<style scoped>
</style>