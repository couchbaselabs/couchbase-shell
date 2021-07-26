<template>
  <pre><code>{{ prettifiedJson(rows) }}</code></pre>
</template>


<script>
export default {
  name: "NotebookCellJson",
  props: ["rows"],
  methods: {
    prettifiedJson(input) {
      return JSON.stringify(input, null, 2);
    },
  },
};
</script>

<style scoped>
</style>