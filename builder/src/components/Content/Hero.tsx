import HeroImage from "assets/images/hero.png";
import type { MouseEventHandler, ReactNode } from "react";
import useAdmin from "services/stores/admin";

interface PropTypes {
  title: ReactNode;
  intro?: ReactNode;
  subTitle?: ReactNode;
  cta?: ReactNode;
  onClick?: MouseEventHandler;
}

export const Hero = ({
  title,
  intro = undefined,
  subTitle = undefined,
  cta = undefined,
  onClick = undefined,
}: PropTypes): ReactNode => {
  const isInBlockAdmin = useAdmin((state) => state.isInBlockAdmin);
  const handleClick = !!onClick ? onClick : undefined;

  return (
    <div
      className={`pt-24 ${isInBlockAdmin ? "hover:outline-blue" : ""}`}
      onClick={handleClick}
    >
      <div className="container px-3 mx-auto flex flex-wrap flex-col md:flex-row items-center">
        <div className="flex flex-col w-full md:w-2/5 justify-center items-start text-center md:text-left">
          {!!intro ? <>{intro}</> : null}

          {title}

          {!!subTitle ? <>{subTitle}</> : null}

          {!!cta ? <>{cta}</> : null}
        </div>

        <div className="w-full md:w-3/5 py-6 text-center">
          <img className="w-full md:w-4/5 z-50" src={HeroImage} alt="Hero" />
        </div>
      </div>
    </div>
  );
};

/*
propertyControls: {
  intro: {
    type: ControlType.ComponentInstance,
    allowedComponents: ["Text"],
    defaultProps: [
      {
        tag: "p",
        fontSize: "base",
        className: "uppercase tracking-loose w-full",
        children: "What business are you?",
      },
    ],
  },
  title: {
    type: ControlType.ComponentInstance,
    allowedComponents: ["Text"],
    defaultProps: [
      {
        tag: "h1",
        fontSize: "5xl",
        className: "my-4 font-bold leading-tight",
        children: "Main Hero Message to sell yourself!",
      },
    ],
  },
  subTitle: {
    type: ControlType.ComponentInstance,
    allowedComponents: ["Text"],
    defaultProps: [
      {
        tag: "p",
        fontSize: "2xl",
        className: "leading-normal mb-8",
        children:
          "Sub-hero message, not too long and not too short. Make it just right!",
      },
    ],
  },
  cta: {
    type: ControlType.ComponentInstance,
    allowedComponents: ["Button"],
    defaultProps: [
      {
        backgroundColor: "white",
        className:
          "mx-auto lg:mx-0 hover:underline text-gray-800 font-bold rounded-full my-6 py-4 px-8 shadow-lg transform transition hover:scale-105 duration-300 ease-in-out",
        children: "Register!",
      },
    ],
  },
}
*/

export default Hero;
