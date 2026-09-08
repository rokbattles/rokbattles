import { domMax, LayoutGroup, LazyMotion, MotionConfig } from "motion/react";
import { type ComponentProps, useId } from "react";

export function NavigationSection(props: ComponentProps<"div">) {
  const id = useId();
  return (
    <MotionConfig reducedMotion="user">
      <LazyMotion features={domMax} strict>
        <LayoutGroup id={id}>
          <div {...props} />
        </LayoutGroup>
      </LazyMotion>
    </MotionConfig>
  );
}
