<script lang="ts">
    import '../app.css';
  </script>
  
  <slot />
  
  <style>
    :global(body) {
      margin: 0;
      padding: 0;
      font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
      overflow: hidden;
    }
    
    :global(*) {
      box-sizing: border-box;
    }
  </style>
  