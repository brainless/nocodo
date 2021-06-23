import loadable from "@loadable/component";

const TopNavigation = loadable(() =>
  import(
    /* webpackChunkName: "topNavigation" */ "components/Navigation/TopNavigation"
  )
);
const Page = loadable(() =>
  import(/* webpackChunkName: "page" */ "components/Page")
);

const Content = () => {
  return (
    <>
      <TopNavigation />

      <Page />
    </>
  );
};

export default Content;
