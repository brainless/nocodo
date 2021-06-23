import BlockLoader from "components/Block/Loader";

const PageEditor = () => {
  return (
    <>
      <div className="mt-14" />

      <EditorBar />
      <BlockLoader blockType="Hero" />
    </>
  );
};

export default PageEditor;
