<?php get_header(); ?>
<main class="wrap">
<?php if ( have_posts() ) : while ( have_posts() ) : the_post(); ?>
  <article class="post-card">
    <?php the_post_thumbnail( 'medium' ); ?>
    <h2 class="post-title"><a href="<?php the_permalink(); ?>"><?php the_title(); ?></a></h2>
    <div class="post-text"><?php the_content(); ?></div>
  </article>
<?php endwhile; endif; ?>
</main>
<?php get_footer(); ?>
