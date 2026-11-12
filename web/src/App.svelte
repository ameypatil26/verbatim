<script lang="ts">
  import Navbar from './lib/Navbar.svelte';
  import Hero from './lib/Hero.svelte';
  import Features from './lib/Features.svelte';
  import HowItWorks from './lib/HowItWorks.svelte';
  import Architecture from './lib/Architecture.svelte';
  import Comparison from './lib/Comparison.svelte';
  import Faq from './lib/Faq.svelte';
  import Footer from './lib/Footer.svelte';
</script>

<a
  href="#main"
  class="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[100] focus:rounded-lg focus:bg-mint focus:px-4 focus:py-2 focus:text-[12px] focus:font-semibold focus:text-ink"
>
  Skip to content
</a>

<Navbar />

<main id="main">
  <Hero />
  <Features />
  <HowItWorks />
  <Architecture />
  <Comparison />
  <Faq />
</main>

<Footer />
