<css-doodle class="animation">
	<style>
		--color: var(--home-box-color);
		--delay: 80s;

		:container {
			perspective: 15vmin;
			transform: scale(0.5) translateY(-10%);
			--deg: @p(-180deg, 180deg);
			opacity: .4;
		}

		@grid: 30x1 / 100% 50% / var(--home-grid-color);
		@place: center;
		@size: 18vmin;

		border: 1px solid @p(--color);
		will-change: transform opacity;
		animation: scale-up @p(--delay) linear infinite;
		animation-delay: calc(-@p(--delay) / @I * @i);

		@keyframes scale-up {
			0%, 95.01%, 100% {
				transform: translateZ(0) rotate(0);
				opacity: 1;
			}
			95% {
				transform:
					translateZ(18vmin) rotateZ(var(--deg));
					opacity: 0;
			}
		}
	</style>
</css-doodle>

<style lang="postcss">
	.animation {
		width: 100%;
		height: 100%;
	}
</style>
